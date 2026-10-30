# 0967. An object spread, a splice and a set by any key

Status: Accepted. Amended: `js::array::join` (below); `js::array::splice` takes numbers; a `Dict` spread, and a tuple of them in order (the user's decision, 2026-10-28); an `f64` is a key (rd101); a spread of `None` written is `{}` (rd210).

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

## Amendment: `js::array::join` of any values

React's hook-deps warning writes `[${prevDeps.join(', ')}]` of any values.
Its port mapped each `Option<&Unknown>` to text, `None` to `""`, then
joined: `deps.map((dep) => (dep != null ? String(dep) : "")).join(", ")`.

**`js::array::join(values, separator)` is `values.join(separator)`** of a
slice of any values, `&[Option<&Unknown>]`: JS's own operation, each
value's text as `String()` gives it, `undefined` and `null` none, a
symbol's `TypeError` a panic. Case N, JS's own on a JS value. Text Rust
owns is joined by `[String]::join`, as before.

Held by `test/js-collections.test.ts`'s "js::array::join is an array of
any values' join", run in Node, and the guide's `deps_text`.


## Amendment: `js::array::splice` takes numbers, as JS's does (2026-10-28)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, JS's own operation
on a JS value, given what the program gives it.

React's copyWithDeleteImpl writes `updated.splice(key, 1)` of a path's
key, any number (gap rc5 of the port). Its start was a `u32`, narrower
than JS's `number`: the port cast it, `at as u32`, so the JS converted it
first, `$f64ToInt(key, 0, 4294967295)`, and a negative key, which JS's
`splice` counts from the end, was `0`: what the Rust said, not what
upstream does.

**`js::array::splice(value, start, delete_count)` takes `f64`s**, as
TypeScript's `splice(start: number, deleteCount: number)` does:
`updated.splice(at, 1)`, JS converting each as it does, a negative
`start` from the end.

Held by `test/js-object.test.ts`'s "an object spread, a splice and a set
by any key are JS's", run of a negative and a fractional start, and the
guide's `copy_with_delete`.

## Amendment: a `Dict` spread, and a tuple of them in order (2026-10-28)

The user's decision (React port gap rd64c): a `Dict` literal with a
leading spread of another `Dict` is this decision's `js::object::spread`,
no new name. Case ([0262](0262-when-rust-and-js-disagree.md)): N for the
form, JS's own spread; C for its order, Rust's kept where JS's would
differ.

React DOM's `SyntheticEvent.js` makes each event interface of another:

```js
const UIEventInterface = {...EventInterface, view: 0, detail: 0};
```

The port wrote it as entries, `dict::from_entries([dict::entries(
EVENT_INTERFACE), vec![("view", ..), ..]].concat())`, which was
`Object.fromEntries([...Object.entries(EventInterface), ["view", 0],
..].map(..))`, and isn't a spread: `Object.entries` skips a symbol key the
spread copies.

**`js::object::spread` takes what it spreads by a trait, `js::object::
Spread`:** any value, `Option<&Unknown>`, a `JsObject`, as before; a
`&Dict<T>`, a `Dict<T>`; and a tuple of two or three of one kind, `{ ...a,
...b }`, each spread in order, a later key's value the one kept where an
earlier one's key is, as JS's spread keeps it:

```rust
static UI_EVENT_INTERFACE: &'static EventInterfaceType = js::object::spread((
    EVENT_INTERFACE.with(|i| *i),
    dict::from_entries(vec![("view", Normalize::Copied(0)), ("detail", Normalize::Copied(0))]),
));
```

```js
const UIEventInterface = { ...EventInterface, view: 0, detail: 0 };
```

- **A spread of an object written out is its fields**, `{ ...a, ...{
  view: 0 } }`'s `{ ...a, view: 0 }` (`prepare::objects`): the object is
  new, so spreading it defines each field where the outer object would, in
  order, each made where it was; a `__proto__` field too, printed
  `["__proto__"]`, its own in both. Not one of a getter or a setter,
  whose value a spread copies.
- **Rust makes a tuple's values before any is spread**, where JS spreads
  each before it makes the next. So where a later one runs the program's
  code, which may change what's spread before it, each is made first, in
  order: `js::object::spread((a, changed(a)))` is `const spread =
  changed(a); return { ...a, ...spread };`, `a` as `changed` left it, as
  in Rust. A `Dict` written out runs none of it: `dict::from_entries` is
  `#[rust_js::runs_nothing]`, as it reads the pairs it's given and calls
  nothing in them. And tidy moves nothing a call makes past a spread of a
  value a call may change (`moves::leads`, an object's spread read as an
  array's `...items` is, `read_past_calls`): `{ ...a,
  ...Object.fromEntries([["c", counted(a)]]) }` would read `a` before
  `counted(a)` sets it.
- **A tuple of one isn't one**: `spread(x)` is the value's, the one form
  of a single spread.

Alternatives:

- **`dict::set` of the spread's copy, folded into the literal**: `{ ...a
  }` then `copy.view = 0` *sets* where the literal defines, which differ
  where `Object.prototype` has a setter or a read-only property of the
  key: case B, not this decision's to take.
- **The entries' concat printed as a spread**: a different operation, of
  a symbol key, as above.
- **A new `dict::spread` or `dict::extend`**: a second name for JS's one
  spread; the user chose this one's.

Held by `test/js-collections.test.ts`'s "a Dict spread before more of it
is JS's object spread": React's interfaces printed as upstream's, a later
key over an earlier, a symbol copied and nothing inherited or not
enumerable, each against JS's own spread, and `a` read after a call that
sets it, both where lowering makes it first and where tidy would move it;
mutations `object-spread-tuple-whole`, `object-spread-parts-made-in-place`,
`object-spread-written-out-held` (`lower/mir/calls.ts`),
`object-spread-read-before-call` (`lower/mir/tidy/moves.ts`) and
`spread-written-out-kept` (`prepare/objects.ts`); the guide's
`ui_event_interface`.

## Amendment: an `f64` is a key (rd101, 2026-10-29)

Case N: JS's own property access, of the number it's given.

React's ReactDOMEventReplaying walks its form replaying queue, a JS array,
three items at a time: `formReplayingQueue[i + 1]`, `i += 3`. A key was
text, a JS value or a `u32`, `i32` or `usize`, so the port's index was a
`usize`, `formReplayingQueue[(i + 1) >>> 0]`, `i = (i + 3) >>> 0`.

**`f64`, TypeScript's `number`, is a `PropertyKey`**, as ADR 0994 makes
WebIDL's numbers `f64`s: `js::get(queue, &(i + 1.0))` is `queue[i + 1]`,
the number as JS has it, `1.5`'s `"1.5"` and `NaN`'s `"NaN"` as JS names
them. Held by `test/js-any-values.test.ts`'s "an f64 is a property key,
its index as JS writes it", run on an array; no lowering changed, so no
mutation: without the impl the test doesn't compile.

## Amendment: a spread of `None` written is `{}` (rd210)

React DOM's animateGesture gives `[{}, {}]` where `effect.getKeyframes()`
gives a `Vec<&JsObject>`: two new objects of nothing. `spread(None)` made
each, which printed `{ ...undefined }`. A spread of `undefined` or `null`
defines nothing (ECMAScript's CopyDataProperties), so **a spread of a
`None` written, a literal `undefined` or `null`, is left out: `spread(None)`
is `{}`**. Case N of ADR 0262: the same object either way. Held by
`test/js-object.test.ts`'s "an object spread, a splice and a set by any
key are JS's" (run: two objects, each `{}`, not one) and mutation
`spread-of-none-written` (`mir/calls.ts`).

