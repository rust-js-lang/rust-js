# 0666. The js crate binds `Object`'s reflection, a descriptor a dictionary

Status: Accepted. Amends [0283](0283-js-crate-covers-typescripts-es-library.md),
whose `PropertyDescriptor` and `ProxyHandler` were made empty, `{}`, and
given what's set, and whose measure counted no `Object`. Amended below: a
dictionary's keys written out are any text.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: each is JS's own
function, given and giving what TypeScript types it as.

## Context

React's `shared/ConsolePatchingDev.js` replaces `console`'s methods and
puts them back:

```js
const props = { configurable: true, enumerable: true, value: disabledLog, writable: true };
Object.defineProperties(console, { info: props, log: props, /* .. */ });
// ..
Object.defineProperties(console, { log: {...props, value: prevLog}, /* .. */ });
```

and `shared/ReactSerializationErrors.js` reads what an object is:
`Object.prototype` compared with `===`, `Object.getOwnPropertyNames`,
`names[i] in ObjectPrototype`, and `Object.getOwnPropertyDescriptor(object,
name)`'s `get` and `enumerable`. The js crate's `object` had `keys`,
`assign`, `fromEntries`, `getPrototypeOf`, `is` and `toString`; the rest
of TypeScript's `ObjectConstructor` wasn't bound, nor `Object.prototype`,
and the measure (ADR 0283) didn't count `Object`, "Rust's own type's".
`Reflect`'s `PropertyDescriptor` was a JS object made empty and given its
fields by setters, `d.set_writable(false)`: `const d = {}; d.writable =
false;`, where JS writes the object, and where webapi's dictionaries and
Intl's options are structs (ADRs 0024, 0283). A second form of a
dictionary, and so was `ProxyHandler`'s.

## Decision

**Each of TypeScript's `ObjectConstructor` statics, ES5 to ES2024, is
the js crate's, in `js::object`, typed as TypeScript types it; and a
property descriptor is one dictionary, a struct, shared by `Object` and
`Reflect`.**

| TypeScript | Rust | JS |
|---|---|---|
| `defineProperty<T>(o: T, p, attributes): T` | `object::define_property(o, key, &PropertyDescriptor { .. })`, `&T` to `&T` | `Object.defineProperty(o, "id", { value: "fixed", writable: false })` |
| `defineProperties<T>(o: T, properties: PropertyDescriptorMap): T` | `object::define_properties(o, &Logs { log: props, .. })` | `Object.defineProperties(console, { log: props })` |
| `getOwnPropertyDescriptor(o: any, p): PropertyDescriptor \| undefined` | `Option<PropertyDescriptor<'static>>` | `Object.getOwnPropertyDescriptor(object, name)` |
| `getOwnPropertyDescriptors`, `getOwnPropertyNames`, `getOwnPropertySymbols` | a `&Dict<PropertyDescriptor>`, a `Vec<String>`, a `Vec<&Symbol>` | each its own |
| `create(o: object \| null, properties?)` | `create(Option<&Unknown>, Option<&dyn PropertyDescriptorMap>)`, `None` `null` | `Object.create(null)` |
| `setPrototypeOf(o, proto: object \| null): any` | `&'a Unknown` to `&'a Unknown`, `None` `null` | `Object.setPrototypeOf(bare, made)` |
| `freeze`, `seal`, `preventExtensions` of `T` | `&T` to `&T` | each its own |
| `isFrozen`, `isSealed`, `isExtensible`, `hasOwn` | `bool` | each its own |
| `values<T>(o: { [s: string]: T })` | `dict::values`, beside `dict::entries` and `dict::keys` | `Object.values(counts)` |
| `groupBy(items, keySelector)` | `group_by(Vec<T>, impl FnMut(&T, u32) -> K) -> &Dict<Vec<T>>` | `Object.groupBy(words, (word) => ..)` |
| `readonly prototype: Object` | `object::prototype`, a `&'static Unknown` static | `Object.prototype` |

- **A descriptor is a dictionary**, as a webapi one is:
  `PropertyDescriptor<'a>` of `Option` fields, `Copy`, `Default`, each
  `None` left out, so what it leaves out stays as the property had it,
  as JS reads what's absent. Its `value` is any value, `Option<&'a dyn
  IntoUnknown>` (ADR 0654); its `get` and `set` the JS functions they are
  (ADR 0653), `&'a dyn Fn() -> Option<&'static Unknown>` and `&'a dyn
  Fn(Option<&Unknown>)`, TypeScript's `get?(): any`, `set?(v: any)`. One
  descriptor's update of another is Rust's struct update, `PropertyDescriptor
  { value: Some(&prev_log), ..props }`, `{ ...props, value: prevLog }`, as
  a `Copy` struct's always was.
- **A descriptor map is any dictionary of descriptors**:
  `PropertyDescriptorMap`, an unsafe marker as `JsonText` is, of a
  `Dict<PropertyDescriptor>` and of a struct of them its `impl` vouches
  for, `unsafe impl PropertyDescriptorMap for Logs<'_> {}`, whose literal is
  the object JS writes, `{ info: props, log: props }`. A `Dict` written
  out, `dict::from_entries(vec![..])`, is the object of them too.
- **`ProxyHandler` is a dictionary too**, of a closure JS keeps for each
  trap, `Option<Box<dyn Fn(..)>>` (ADR 0260): `ProxyHandler { get:
  Some(Box::new(..)), ..Default::default() }`. `property_descriptor::new`
  and `proxy_handler::new`, and the setters, are gone: one form of a
  dictionary.
- **`key in object` is `js::has`**, as it was (ADR 0225); `Object.hasOwn`
  is `object::has_own`. `Object.prototype.toString.call` stays
  `object::to_string`, and `Object.is` `object::is`.
- **`Object` is measured**: `builtins/coverage.ts` counts its
  constructor's statics, 23, all bound, 712 of 712 now. Its own members,
  `toString` and `hasOwnProperty`, are every value's, and `new Object()`
  a struct's: not counted.

## Why

- **TypeScript's members, each typed as it types them**: React's two
  files are written in them, and a port binding its own would be a
  stand-in for what the crate should bind.
- **One form of a dictionary**: a struct, as webapi's, Intl's and every
  options object a program declares are. The setter form wrote a
  statement for each field, where JS writes an object; and a second form
  is a choice an agent gets wrong.
- **A struct update is a spread already**: the shape React writes,
  `{...props, value: prevLog}`, is what Rust's `..props` of a `Copy`
  struct compiles to, with nothing new.

## Alternatives

- **Keep the setter form beside a struct**: two ways to make one object.
- **A descriptor map of `Dict` alone**: `Object.fromEntries([["info",
  props], ..])` is no object literal; a struct is, and its fields are
  checked by rustc.
- **`value: Option<&'a Unknown>`**: a Rust value, a function item, would
  need a cast to be given, where `dyn IntoUnknown` takes any (ADR 0654).

## Consequences

- **A descriptor's `value` read back is a `dyn IntoUnknown`**, which a
  program can't read as a JS value: what `getOwnPropertyDescriptor` gives
  is read for its `get`, `set`, `enumerable`, `configurable` and
  `writable`. `reflect::get` reads the property itself.
- **`Object.defineProperty`'s throw is a panic**, as a binding's that
  doesn't say it throws is; `reflect::define_property` gives `false`.
- **`Object.create`'s `Some(object::prototype)` is
  `Object.prototype`**, as a nullable parameter's `Some(..)` is its value
  (ADR 0275's amendment); `None` is `null`.
- A key `group_by`'s closure gives is a `PropertyKey` by value, a
  `String` or a number: no `&str` a closure could borrow.

## What holds it

- `test/js-object.test.ts`, "Object's statics are JS's, a property
  descriptor a dictionary": each static, its JS and its value run; and
  "React's serialization errors read an object as upstream does":
  `isObjectPrototype`, `isGetter` and `objectName` as React writes them,
  run on objects of this realm and of none.
- `test/js-builtins.test.ts`, "Reflect and Proxy are JS's": a descriptor
  and a handler written as the objects they are.
- `test/builtins-coverage.test.ts`: `Object`'s 23 statics.

## Amendment (2026-10-28): a dictionary's keys written out are any text

react-dom-bindings' data modules are object literals of string keys their
consumers index, validAriaProperties' `{'aria-current': 0, // state ..}`,
possibleStandardNames' `{accept: 'accept', 'accept-charset':
'acceptCharset', ..}`. Their port is a `Dict` written out, which is the
object literal (above), but `dict::from_entries` took `Vec<(String, T)>`,
so each of hundreds of keys was `"aria-current".to_string()`, where
`HashMap::from` takes `("aria-current", 0)`, and TypeScript's
`Object.fromEntries` any `PropertyKey`. The port didn't find the form.

**Its keys are any text, what a `str` takes (`js::Takes`): a `&str`, a
`&String` or a `String`**, `dict::from_entries(vec![("aria-current", 0),
..])`, all of them a JS string, so the JS is as before:

```js
const ariaProperties = {
  "aria-current": 0, // state
  // Widget Attributes
  "aria-autocomplete": 0,
};
```

- **`Takes` says what text is**, as it does of a rest parameter's
  arguments (ADR 0640): one trait, its message now "a parameter of
  `str`s takes no `u32`", not a rest's alone.
- **The guide shows the form**, beside `js::Dict`
  (interop/types.md), where the port looked for it and found none.
- Case N ([0262](0262-when-rust-and-js-disagree.md)): a key's text is
  the same string either way.

Held by `test/lowering-strings.test.ts`, "a dictionary of entries written
out is an object literal": `&str` keys and a comment between entries, a
`&String` key, run; and the guide's example. A binding's signature has no
mutation: the test that calls it with `&str` doesn't compile without it.

