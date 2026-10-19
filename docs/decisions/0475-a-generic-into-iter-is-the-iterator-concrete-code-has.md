# 0475. A generic `into_iter()` is the iterator concrete code has

Status: Accepted. Extends [0061](0061-generic-iterators.md) and
[0473](0473-fuse-is-rusts-fuse.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): one representation of
a value, wherever it's made.

## Context

A generic `IntoIterator` is given as its caller has it: a `Vec` as its
array, a map or a set as itself (ADR 0061). `i.into_iter()` of one was
`Iterator.from(i)`, JS's iterator of it. But concrete code holds a `Vec`'s
`IntoIter` as a `$iter` of its array, which knows where it is, and reads its
`len()`, `as_slice()`, clone and exact `size_hint()` from that. One a
generic function made and gave back, `mk(vec![1, 2, 3]).len()`, was `NaN`,
and its clone threw.

## Decision

**`into_iter()` of a generic `IntoIterator` is `$intoIter(i)`**: of an
array, the `$iter` concrete code has; else JS's iterator of it, which is
what concrete code has of a map's or a set's.

- **Stepped in place, where nothing reads where it is**, it's JS's own:
  `Iterator.from(i).map(f)` of a helper's or a consumer's, and `i` of a
  `for`'s, tidy's `consumed_fuses` as of a `Fuse` (ADR 0473).

```js
function mk(i) {
  return $intoIter(i);
}

function show(items) {
  for (const x of items) {
    console.log(`shown ${x}`);
  }
}
```

## Why

- **One value, wherever it's made**: `generic_into_iter_values` reads the
  `len()`, `size_hint()`, `as_slice()` and clone of a `Vec`'s `IntoIter` a
  generic function gave back, and of each in a struct, with native Rust.

## Consequences

- **What was wrong is right**: a `Vec`'s iterator a generic function makes
  and gives back. Its `size_hint()` in generic code is exact, as std's is.
- **A generic iterator kept in a variable is `$intoIter(items)`**, where
  `Iterator.from(items)` was.
