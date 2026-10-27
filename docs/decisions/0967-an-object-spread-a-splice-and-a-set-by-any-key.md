# 0967. An object spread, a splice and a set by any key

Status: Accepted.

Case ([0262](0262-when-rust-and-js-disagree.md)): N for each, JS's own
operation on a JS value, as TypeScript has it of an `any`'s.

## Context

React's DEV-only DevTools helpers in ReactFiberReconciler.js copy a value
with one path changed:

```js
const copyWithDeleteImpl = (obj, path, index) => {
  const key = path[index];
  const updated = isArray(obj) ? obj.slice() : {...obj};
  if (index + 1 === path.length) {
    if (isArray(updated)) {
      updated.splice(key, 1);
    } else {
      delete updated[key];
    }
    return updated;
  }
  updated[key] = copyWithDeleteImpl(obj[key], path, index + 1);
  return updated;
};
```

`copyWithRename` and `copyWithSet` are its kin. The values are any JS
values, `Object | Array<any>`, their keys `string | number`. The js crate
had `js::delete`, `js::get` of any key and `Kind::Array`'s items, whose
`to_vec()` is `slice()`, but no `{...obj}`, no `splice` of a JS value,
and `js::set` took a `&str` key alone.

`Object.assign({}, obj)` isn't `{...obj}`: `assign` *sets* each property,
so an own `__proto__` key, `JSON.parse('{"__proto__": ..}')`'s, sets the
copy's prototype, where a spread *defines* it, an own property like any
other.

## Decision

Three extensions of what the js crate has, each JS's one form:

- **`js::object::spread(value: Option<&Unknown>) -> &'static JsObject`**
  is `{ ...value }`: `link_name = "{...}"`, the object form `"{a, b}"`
  (an object of its arguments, in order) with `...` meaning the argument
  spread in, `Prop::Spread`. Of `None`, `{}`, as JS spreads `undefined`
  and `null`.
- **`js::array::splice(this: &Unknown, start: u32, delete_count: u32) ->
  Vec<Option<&'static Unknown>>`** is `this.splice(start, n)`, of any
  value an array, what it removed; of one with no `splice`, JS's
  `TypeError`, a panic. An array the program owns is a `Vec`, whose
  `remove` and `drain` say this; `splice` is of a JS value's array.
- **`js::set(value, key, to)` takes `key: &K` of any `PropertyKey`**, as
  `js::get`, `js::has` and `js::delete` do: text, a number or a JS value,
  `updated[key] = ..` of a path's `string | number`.

```rust
let updated: &Unknown = match js::classify(Some(obj)) {
    Kind::Array(items) => js::unknown_of(items.to_vec()),
    _ => js::object::spread(Some(obj)),
};
```

```js
const updated = Array.isArray(obj) ? obj.slice() : { ...obj };
```

## Why

- **It's upstream's JS**, each operation as React writes it.
- **One form each**: a spread is the object form's `...`, not a new
  attribute; `splice` a method of a value, `this`'s, as `hasOwnProperty`
  is; `set`'s key `get`'s.
- **It runs the same**: each is JS's own operation; Rust is told what
  each gives.

## Alternatives

- **`js::object::assign(js::object::from_entries(vec![]), (obj,))`** for
  the copy: a different operation of a `__proto__` key. Rejected.
- **A `Vec` for the array branch, `remove(at)`**: Rust's own array is a
  `Vec`, but `updated` is the JS value both branches give, set by key
  after; and `remove` panics past the end, where `splice` removes none.

## Consequences

- `js::set`'s signature is generic in its key; callers of a `&str` are
  unchanged.

## What holds it

- `test/js-object.test.ts`'s "an object spread, a splice and a set by any
  key are JS's", copyWithDelete run in JS, a `__proto__` key's copy too;
  the guide's example, compiled.
- Mutation `object-binding-spread-a-field` (`mir/calls.ts`).
