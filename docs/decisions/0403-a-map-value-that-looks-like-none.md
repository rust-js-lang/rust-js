# 0403. A map's value that could look like `None` is given back boxed

Status: Accepted. Extends [0051](0051-generic-options.md) and
[0059](0059-hashmap.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): `Some(())` and
`None` are two answers; a JS `undefined` for both would change results.

## Context

A map of `()`s, a set by another name, or of `Option`s, gives back an
`Option` of its value from `get`, `insert` and `remove`. `m.get(k)` is
`undefined` both where nothing's there and where `()` or `None` is, so
these were refused, as any call whose `Option` could hold what looks like
`None` is (ADR 0051). rustc's `unsized/issue-23649-2.rs` failed on it.

## Decision

**A map's `get`, `insert` and `remove` give `Some` of its value boxed
where the value looks like `None`, as `$some` boxes it: `$someGet(m, k)`
for `get`; `$insert` and `$remove` box what they give back always, which
`$some` leaves as it is unless it looks like `None`.**

```js
const value = $insert(units, "a");
console.log(`${$someGet(opts, 1)}`);
```

- **Of any other value, nothing changes**: `m.get(k)` is still `get`'s,
  and `$some` gives a value that doesn't look like `None` back as it is.
- **`get_mut`** is the handle it was (ADR 0152), which nothing mistakes
  for `None`.

## Why

- **It's the program Rust runs**: the corpus's `map_values_like_none.rs`
  inserts, gets, removes and changes `()`s in a `HashMap` and `Option`s in
  a `BTreeMap`, each with native Rust; the rustc test passes.
