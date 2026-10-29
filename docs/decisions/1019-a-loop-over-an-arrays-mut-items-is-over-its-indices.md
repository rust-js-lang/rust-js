# 1019. A loop over an array's `&mut` items is over its indices

Status: Accepted (#320, #354).

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: each item is read and written in its place, as its handle read
and wrote it, in the same order, for as many turns.

## Context

`for s in state.iter_mut() { *s += 1; }` printed `for (const s of
$mutItems(state)) { s.value = (s.value + 1) >>> 0; }`, a handle on each
item made only to be written through. Its `zip` with another array,
TheAlgorithms' SHA-3 sponge, was `for (const [s, c] of
$lazyZip($iter($mutItems(state)), chunk.slice()))`, and its `enumerate()`,
javascript-algorithms' Levenshtein distance, `for (const [i, cell] of
$mutItems(row).entries()) { cell.value = i; }`. Upstream writes the index
loop, `for (let i = 0; i <= a.length; i += 1) { distanceMatrix[0][i] = i; }`.

## Decision

**A loop over an array's items through their handles, each read and
written only as `s.value`, is a loop over its indices** (`tidy::mut_items`,
after the loops are made, ADR 0974's form):

```js
for (let i = 0; i < state.length; i++) {
  state[i] = (state[i] + 1) >>> 0;
}
```

- **Of its `enumerate()`**, the index is the loop's, `row[i] = i`.
- **Of its `zip` with another array**, `b.slice()`, the copy the zip owns,
  or `b.iter()`'s `$iter(b)`, as long as the shorter, asked once before
  the loop as Rust's `Zip` asks it, each item of the other read at the
  turn's top: `const n = Math.min(state.length, chunk.length); for (let i
  = 0; i < n; i++) { const c = chunk[i]; .. }`. Not of a variable alone,
  `zip(from)` of a slice, which tidy can't tell from one holding an
  iterator, nor of what a zip drops, `$closing(..)`.
- **A handle given on, `double(v)`, or kept, keeps its form**, as only a
  handle is the item's place there.

## Why

- **It's the JS a person writes**, and what upstream's ports write.
- **Nothing can tell**: borrowck keeps the loop's body from reaching the
  array but through the handle, so its length, and the other array's,
  don't change; `state[i]` reads and writes what `s.value` did. A closure
  in the body reads `state[i]` of its own turn's `i`, as JS's `let` gives
  each turn its own.

## Alternatives

- **`state.forEach((s, i, a) => ..)`**: a callback, where a `break` or a
  `return` of the loop can't go.
- **In the loop's lowering**: the handle is MIR's `&mut`, its uses only
  known once the body is lowered; tidy reads them where they are.

## Consequences

- Held by `test/corpus/iter_mut_index_loops.rs` against native Rust: each
  form, a `break`, a closure of the item, a handle given on, a zip of a
  shorter array and of a slice's `iter()`. Mutations in
  `scripts/mutations/lower/mir/tidy/mut_items.ts`.
- The array's and the other's assignments in the body are checked, which
  borrowck rules out: defense no program reaches, with no mutation.
