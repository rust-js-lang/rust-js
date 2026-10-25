# 0904. A `for` over a slice or a range from its end counts down

Status: Accepted. Amends [0871](0871-rev-is-stds.md), whose `rev()`
stepped where it's made a `for` loop read as `for (const x of
v.toReversed())`, a copy of the items reversed that a loop no longer
makes.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, nothing the program
can observe. Rust lends the slice to the iterator for the whole loop, so
nothing changes it while the `for` reads `v[i]`; a range's end is read
once, as Rust reads it making the range, and its start, which the test
reads each step, is one that reads the same each time.

## Context

React's `getComponentNameFromFiber` reads its debug info from the end:

```js
for (let i = debugInfo.length - 1; i >= 0; i--) {
  if (typeof debugInfo[i].name === 'string') { .. }
}
```

Its port writes the iterator clippy's `needless_range_loop` asks for,
`for info in debug_info.iter().rev()`, which was `for (const info of
debugInfo.toReversed())`: a copy of the array made to step it, which no
person writes. A range from its end, `for n in (1..=3).rev()`, was `for
(const n of $iterator({ start: 1, end: 3 }, $rangeInclusiveNextBack))`, a
helper where `for (let n = 3; n >= 1; n--)` says it.

## Decision

**A `for` over an array's items reversed, `rev()` of a slice's `iter()`
or a `Vec`'s `into_iter()`, counts its indices down, each item read in
place, of a variable the body doesn't assign; a `for` over a range of
numbers stepped from its end counts down from its end, of a start that
reads the same each step.** `js::StmtKind::For` counts `down`, `i--`.

```rust
for x in v.iter().rev() { .. }
for i in (a..b).rev() { .. }
for i in (a..=b).rev() { .. }
```
```js
for (let i = v.length - 1; i >= 0; i--) { const x = v[i]; .. }
for (let i = b - 1; i >= a; i--) { .. }
for (let i = b; i >= a; i--) { .. }
```

- **What's reversed is the iterators' form**: `iterators::reversed` makes
  `items.toReversed()`, `iterators::reversed_source` reads it, and tidy's
  loop (`tidy/loops.rs::from_the_end`) asks it, as it asks
  `stepped_range` of a range's.
- **The index is `i`**, or a name nothing in the body has, `i$1`; the
  item bound as the `for…of` bound it, `const`, `let` or destructured.
- **Not counted down, still**: a range of `char`s, text that `--` doesn't
  step, nor a `BigInt`'s; a start the body assigns, which Rust read once;
  an array that isn't a variable, `$mutItems(v).toReversed()` of
  `iter_mut()`, an `enumerate().rev()`'s, a field's.

## Why

- **It's what a person writes**, React above all: a loop from the end by
  index, no copy.
- **Rust's loop is the same loop**: the slice lent for its length, a
  range's bounds as Rust made them.

## Alternatives

- **`toReversed()` kept**, a copy of each array a loop steps backwards:
  correct, and what no JS developer writes.
- **`[...v].reverse()`**: a copy too.

## Consequences

- One corpus case, `boxed_array_iter`, and one page of the guide,
  `interop/async.md`'s countdown, read their loops from the end; a page
  of the guide shows a slice's.

## What holds it

- `test/corpus/rev_loops.rs`, beside native Rust: slices of numbers,
  strings and pairs, owned and lent, `break`, `continue` and a label;
  ranges exclusive, inclusive, empty and negative, one whose start the
  body changes, one of `char`s.
- Mutations in `scripts/mutations/lower/mir/tidy/loops.ts` and
  `scripts/mutations/lower/mir/iter/backs.ts`.
