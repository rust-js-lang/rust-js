# 0748. A `for` of a slice's first items is of its part, `xs.slice(0, n)`

Status: Accepted. Extends [0036](0036-iterators-and-sorting.md)'s
`take(n)` and `skip(n)` of an array's items, `xs.slice(0, n)` and
`xs.slice(n)`, from a chain to a `for`.

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the program can't
observe it. The part's items are read as the loop starts, where the
iterator would read each as it's stepped, and nothing changes the array
meanwhile: Rust lends it to the iterator for the whole loop.

## Context

React's `getArrayKind` and `addValueToProperties`
(shared/ReactPerformanceTrackProperties.js) count over at most a hundred
items:

```js
for (let i = 0; i < array.length && i < OBJECT_WIDTH_LIMIT; i++) {
  const value = array[i];
```

The port writes the iterator, as clippy's `needless_range_loop` asks of
an index loop over a slice:

```rust
for &value in array.iter().take(OBJECT_WIDTH_LIMIT) {
```

and rust-js wrote `for (const value of array.values().take(OBJECT_WIDTH_LIMIT))`:
an iterator helper of ES2025, where a chain's `take`, `xs.iter().take(3)
.copied().collect()`, is already `xs.slice(0, 3)`, and `rev()` and `len()`
of `$iter(xs).drop(1).take(2)` read `xs.slice(1).slice(0, 2)` (`sliced`).

## Decision

**A `for` of an array's items taken from, `take(n)`, `skip(n)` or both,
is a `for…of` of the array's part**, by the one rule `sliced` has of
them: `for (const x of xs.slice(0, n))`, `xs.slice(n)`. Of their
`enumerate().take(n)`, the part's `entries()`, whose indices are the
items' own: `for (const [i, x] of xs.slice(0, n).entries())`. Of their
`enumerate().skip(n)`, the iterator's `xs.entries().drop(n)`, as the
part's indices would start at 0.

So React's loops read:

```js
for (const value of array.slice(0, OBJECT_WIDTH_LIMIT)) {
```

## Why

- **It reads as a person writes a part**: `slice(0, n)` is the JS
  everyone knows, where `values().take(n)` is an iterator helper few
  have met, and newer.
- **One rule of an array's first items**: a chain's `take`, a `len()`'s
  and a `rev()`'s of them, and now a `for`'s, each `sliced`.

## Alternatives

- **Upstream's counter**, `for (let i = 0; i < array.length && i < n;
  i++) { const value = array[i]; .. }`: a name the program didn't write,
  and a statement for what it wrote as one expression; the loop of the
  program is of the items.
- **`values().take(n)`, as before**: reads each item as it's stepped, and
  copies none, but is a second form of a part, beside the chain's.

## Consequences

- The part is a copy of up to `n` items, made as the loop starts: a loop
  that leaves early has read the rest too. It's a reference each, at most
  the array's length.

## What holds it

- `test/lowering-iterators.test.ts`, "a for of a slice's first items is
  of its part", run in JS: `take`, `skip`, `enumerate().take` and
  `enumerate().skip`, and React's two loops in the port.
- Mutations `for-of-part-not-sliced` and `for-of-enumerated-part-kept`.
