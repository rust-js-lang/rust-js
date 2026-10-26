# 0910. An object or an array is one value: `kind.object()`

Status: Accepted. Amends [0677](0677-a-weak-key-is-what-kind-holds.md),
whose `Kind::Object(_) | Kind::Array(_)` was no one key, and extends
[0668](0668-a-match-of-what-a-value-is-is-a-switch-of-its-typeof.md)'s
test of them, `typeof value === "object" && value !== null`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. It's the value
itself where it's an object or an array, and `None` where it isn't, as
the or-pattern tests; the `if` of it is that test.

## Context

React's `ReactCapturedValue` keys a `WeakMap` by any object it's given,
an array too:

```js
if (typeof value === 'object' && value !== null) {
  const existing = CapturedStacks.get(value);
  if (existing !== undefined) {
    return existing;
  }
  ..
  CapturedStacks.set(value, captured);
```

`Kind::Object` holds a `&JsObject`, which keys a weak map, and
`Kind::Array` a slice, `[Option<&Unknown>]`, which keys one of its own,
`WeakMap<[T], V>` (ADR 0677). A map of both is `WeakMap<dyn WeakKey, V>`,
and a slice can't be one: it isn't sized, so `&[T]` is no `&dyn
WeakKey`.

```rust
let key: &dyn WeakKey = match js::classify(value) {
    Kind::Object(object) => object,
    Kind::Array(array) => array, // E0277: `[Option<&Unknown>]` isn't sized
    _ => return ..,
};
```

An or-pattern binds one type in each alternative, so `Kind::Object(o) |
Kind::Array(o)` can't name both, and the guard that reads an arm's value,
`Kind::Object(_) | Kind::Array(_) if let Some(value) = value` (ADR 0790),
binds an `&Unknown`, which a string may be. The port cast, `unsafe {
js::cast::<&JsObject>(value) }`.

## Decision

**`kind.object()` is what `typeof value === "object" && value !== null`
takes, as one value: an `Option<&JsObject>` of an `Object`'s or an
`Array`'s, the value itself, TypeScript's `object` it narrows to, which
keys a `WeakMap<JsObject, V>`.** `if let Some(object) =
js::classify(value).object()` is that `if`, its `object` the value:

```rust
if let Some(object) = js::classify(value).object() {
    let existing = CAPTURED_STACKS.with(|captured_stacks| captured_stacks.get(object));
    if let Some(existing) = existing {
        return existing;
    }
    ..
    CAPTURED_STACKS.with(|captured_stacks| captured_stacks.set(object, Rc::clone(&captured)));
```

```js
if (typeof value === "object" && value !== null) {
  const existing = CapturedStacks.get(value);
  if (existing != null) {
    return existing;
  }
  ..
  CapturedStacks.set(value, captured);
```

- **It's a binding's form, `typeof object`**: `typeof value === "object"
  && value !== null ? value : undefined`, the test `untagged.rs`'s
  `object_test` makes of its untagged enum's object and array variants,
  as their or-pattern's is, of the value read once.
- **An `Option` made where it's tested is its test**: tidy's `tested
  options` makes `const x = t ? v : undefined; if (x) { a }`, of a `v`
  that's there, `if (t) { a }`, `x` read as `v`, where `v` is a variable
  nothing changes and `x` is read nowhere else; of a `let .. else`,
  `if (!x) { return; } rest`, `if (!t) { return; } rest`. And its
  presence read in place, `.is_some()`, is `t`. `bool::then_some(v)`'s
  `v`, of a `Some` not boxed, is there too, so `if let Some(x) =
  wanted.then_some(item)` is `if (wanted)`, `x` read as `item`. An `Option`'s presence
  negated, `!x`, is a presence test still (`Known::Presence`), as `!!x`
  is.
- **The guard stays the way to read the value of an arm of a `match`**
  of several kinds, `Kind::Null => .., Kind::Object(_) | Kind::Array(_)
  if let Some(value) = value => ..`, an `&Unknown`; `object()` is the one
  test of object-ness that gives an object.

## Why

- **It's the program's test, as one value**, what upstream's `typeof`
  narrows to, so no cast: a cast is a promise a program can get wrong.
- **It's the Rust a person writes of an enum**: a method that gives a
  variant's view, `serde_json::Value::as_object()`'s shape, of the two
  variants JS calls objects.
- **No type changes**: `Kind::Array` still holds a slice, which its slice
  patterns, `Kind::Array(&[first, _])`, and `items.len()` read.

## Alternatives

- **A sized `Kind::Array` payload, a JS array type that derefs to a
  slice**: a `&dyn WeakKey` of either arm, but `Kind::Array(&[first, _])`
  is refused, as no pattern reads through a `Deref`.
- **`AsRef<JsObject>` of a slice**, `Kind::Array(array) => array.as_ref()`:
  a `let key = match ..` of both arms, whose JS is a variable of each
  arm's value, not upstream's `if`, and a conversion of every Rust slice,
  which isn't always the array JS holds.
- **A `WeakMap` keyed by any `Unknown`**: a string is one, and
  `WeakMap.prototype.set` throws of it (ADR 0677).
- **A function of any value, `js::object(value)`**: a second test of
  what a value is, beside `js::classify`'s `Kind`.

## Consequences

- An or-pattern of `Kind::Object(_) | Kind::Array(_)` that binds the
  value and keys a weak map writes `kind.object()` instead of its guard.
- ADR 0677's consequence, "no one key", is lifted for a map of objects,
  `WeakMap<JsObject, V>`; `dyn WeakKey` still holds no slice.

## What holds it

- `test/js-object.test.ts`, "an object or an array is one weak key,
  kind.object()": React's `createCapturedValueAtFiber`, its `if` and
  `CapturedStacks.get(value)`, run in Bun, an `Error` and an array each
  found again, a string not; and `object().is_some()` of each
  kind.
- The `tested_options` corpus case, against native Rust: `if let` and
  `let .. else` of `bool::then_some`, whose value is there too, each its
  test; and where the value may be `None`, changes where the `Option` is
  read, or the `Option` is read past its test, the `Option` kept.
- Mutations `kind-object-any-value`, `tested-option-undefined-value`,
  `tested-option-changed-value`, `tested-option-read-elsewhere`,
  `presence-of-a-made-option`, `negated-presence-untested`.
