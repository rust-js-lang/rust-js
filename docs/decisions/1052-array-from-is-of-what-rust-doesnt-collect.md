# 1052. `Array.from` is of what Rust doesn't collect itself

Status: Accepted (rd501, P2, 2026-11-01; the coordinator's choice under
the one-way rule). Extends [0860](0860-the-iterator-protocol-and-a-loop-of-what-its-test-makes.md)'s
`Iterable` to a value of any shape, a typed array and a string.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. `Array.from(items)`
is JS's own, and its array is what JS steps out of `items`. Nothing of
Rust's differs.

## Context

React's Flight reply client serializes a JS iterator by copying it into
an array:

```js
const iterator = iteratorFn.call(value);
if (iterator === value) {
  const partJSON = serializeModel(Array.from(iterator as any), iteratorId);
  ..
}
return Array.from(iterator as any);
```

Its Map and Set cases write `Array.from(value)` too. The js crate had no
`Array.from`, so the port could not write the iterator case. A JS iterator
held as a value of any shape is not one Rust steps.

Rust already collects some of these values itself.
`set.values().collect::<Vec<_>>()` is `Array.from(set)`, and
`map.entries().collect()` is `Array.from(map)`, because `collect` of a
collection's own iteration prints that (`#[rust_js::iterates_receiver]`).
A `Vec`, a slice or an array is `to_vec()`.

## Decision

**`js::array::from(items: &dyn Iterable<T>) -> Vec<T>` is
`Array.from(items)`**, typed as TypeScript's `Array.from(iterable:
Iterable<T> | ArrayLike<T>)`. **`js::array::from_with_map_fn(items,
map_fn: impl FnMut(T, f64) -> U, this_arg: Option<&dyn IntoUnknown>) ->
Vec<U>`** is its `mapfn` and `thisArg` overload, `(v: T, k: number) => U`.
Rust can't make `U` default to `T` when no `mapfn` is given, so the
overload is a function of its own, named for its parameter, as
`new_with_buffer` is.

To keep its type as wide as TypeScript's, `Iterable` gains three impls:

- `Unknown`, whose items are `Option<&'static Unknown>`, any value,
  `undefined` too. This is TypeScript's `any`.
- each typed array, of its elements' type, `Iterable<i16>` of an
  `Int16Array`.
- `&str`, of `String`, one per code point, as JS steps a string.

**Each case still has one way (the one-way rule).** What Rust collects
itself is collected, and `from` of it is refused with an error that names
how: `set.values().collect()` for a JS `Set`, `map.entries().collect()` for
a `Map`, `to_vec()` for a `Vec`, a slice or an array. The binding is
marked `#[rust_js::collects]`, and `bindings/collects.rs` refuses a call of
any binding so marked, by the type of what it's given before it's made a
`&dyn Iterable`.

The port's iterator case is then `js::array::from(iterator)`, and its Map
and Set cases cast the `&Unknown` and collect it.

## Alternatives

- **TypeScript's type with no refusal.** A Set or a Map would then have
  two ways to become an array. Rejected (the one-way rule).
- **Only a value of any shape and a JS iterable.** That is narrower than
  TypeScript's type. Rejected.
- **One function whose `mapfn` is optional.** `U` would have no type when
  no `mapfn` is given, so every such call would have to name it.

## Consequences

- The Flight reply client's iterator case is written as upstream's is.
- `new Set(values)` and `new Map(entries)`, which also take an
  `Iterable`, now take a value of any shape, a typed array and a string,
  as TypeScript's do.
- Tests: `test/js-collections.test.ts` "Array.from is of any iterable,
  mapped, and refused of what Rust collects" (compiled, run in Node, and
  the four refusals). Mutation: `array-from-collected-unrefused`.
