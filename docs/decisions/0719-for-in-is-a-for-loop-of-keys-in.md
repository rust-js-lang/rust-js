# 0719. `for…in` is a `for` loop of `js::keys_in`, and its `enumerate()` a counter

Status: Accepted. Extends [0025](0025-vec-loops-refcell-mut.md)'s `for` loops and
[0666](0666-the-js-crate-binds-objects-reflection.md)'s object reflection.
Amended: `keys_in` takes any value, an `Option` too, as JS's loop does.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: the keys are JS's
`for…in`'s, in its order, each as the loop reaches it; and the counter is
the count `enumerate()` gives, where nothing can tell one `let` from a
`const` each turn.

## Context

React's `shared/ReactPerformanceTrackProperties.js` diffs two objects'
keys, their prototypes' enumerable ones too:

```js
let prevPropertiesChecked = 0;
for (const key in prev) {
  if (prevPropertiesChecked > OBJECT_WIDTH_LIMIT) { .. break; }
  ..
  prevPropertiesChecked++;
}
```

The js crate had `Object.keys`, an object's own keys, and nothing of
`for…in`'s: the port's loops left out what an object inherits.

## Decision

**`js::keys_in(object: &Unknown) -> Box<dyn Iterator<Item = String>>` is
the keys `for…in` visits**, a binding of the form `"for in"`:

- **A `for` loop of it is `for (const key in object)`**: tidy's `for…of`
  of it (`loops.rs`), its keys each as JS's loop reaches them, one deleted
  before it's reached not visited, one a nearer key of the same name hides
  not twice.
- **A `for` loop of its `enumerate()`, `for (n, key) in
  js::keys_in(o).enumerate()`, is a counter each turn ends with**: `let n =
  0;` before the `for…in`, `n++` at the end of its body and before each
  `continue` of the loop's, as upstream counts. Only where the counter is
  the same `n` each turn: no closure in the body reads it, which would see
  it change where each turn's `const n` is its own. Else it's the
  iterator's `enumerate()`, below. Two loops' counters of one name are
  named apart, `n` and `n$1`, as any two `let`s of a scope are.
- **Anywhere else it's a JS iterator of them**, `$keysIn(object)`, a
  generator of a `for…in`: `.collect()` is `$keysIn(o).toArray()`, and an
  `enumerate()` a closure reads, `$keysIn(o).map((x, i) => [i, x])`.

`js::StmtKind::ForOf` has `keys`, which the printer writes `in` of: a loop
as `for…of` is, to every pass that walks one, its object read once.
`iterators.rs` owns `$keysIn`'s form, `keys_in` and `keys_in_source`.

```rust
for (prev_properties_checked, key) in js::keys_in(prev).enumerate() {
    if prev_properties_checked > OBJECT_WIDTH_LIMIT {
```
```js
let prevPropertiesChecked = 0;
for (const key in prev) {
  if (prevPropertiesChecked > OBJECT_WIDTH_LIMIT) {
```

## Why

- **The JS a person writes**, upstream's, of the loop a Rust program
  writes with clippy's `enumerate()` (its `explicit_counter_loop` lint asks
  for one in place of upstream's counter).
- **One binding, one meaning everywhere**: the keys are the same in a
  loop and out of one; only how they're written differs, as a `for` of a
  range is `for (let i = ..)` and its iterator `$range`.

## Alternatives

- **`js::object::keys` with the prototypes' added by hand**: a walk of
  `getPrototypeOf` a port would write, and not the JS's.
- **An eager array of the keys**, a helper's: one deleted while the loop
  runs would still be visited.
- **Refuse `keys_in` outside a `for` loop**: a rule that works in one place
  and not the next.
- **A `for…of` of `$keysIn(o)`'s `entries()`-like pairs for the
  `enumerate()`**: a helper and an array each turn, where upstream counts.

## What holds it

- `test/js-object.test.ts`, "a for loop of keys_in is JS's for…in, its
  enumerate a counter": inherited, shadowed, hidden and `_` keys, one
  deleted mid-loop, the counter at a `continue` and at a body's end, a
  closure that reads the count, two counters of one name, and `collect()`.
- Mutations `keys-in-loop-of`, `keys-in-counter-closed-over`,
  `keys-in-continue-uncounted`,
  `keys-in-end-uncounted`, `for-in-printed-of`, `keys-in-own-keys`.
- The guide's [JS values](../guide/interop/types.md#js-values-jsobject-and-unknown).

## Amendment: of any value, an `Option` too (2026-10-28, React port gap rd71)

React DOM's CSSPropertyOperations loops `for (const styleName in styles)`
of styles that may be `null` or `undefined`, which JS's `for…in` visits
nothing of. `keys_in` took an `&Unknown`, so the port wrapped its loop in
`if let Some(styles) = styles`, printed `if (styles != null) { for (..)
}`. **`keys_in` takes any value, `impl IntoUnknown`, as `js::get` and
`js::has` do**: `js::keys_in(styles)` of an `Option<&Unknown>` is `for
(const key in styles)`, and of a `None`, as JS's loop, no key; its
iterator elsewhere, `$keysIn(styles)`, the same. Case
([0262](0262-when-rust-and-js-disagree.md)): None, JS's loop as it is.
Held by `test/js-object.test.ts`, "keys_in of an Option visits none of
None", run in Node.
