# 0398. An array some of whose items moved drops what's left, as rustc does

Status: Accepted. Extends [0098](0098-destructors.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): each value left is
dropped once, in the order native Rust drops it.

## Context

`fn pick([a, b, c, d]: [Noisy; 4], i: usize) -> Noisy` moves one item out
and drops the rest; `let [_, _, m, ..] = items;` moves one and leaves five.
rust-js took apart what's left of a struct, a tuple, a closure or an enum
some of which had moved (rustc's `open_drop`), but not of an array, and
refused it. rustc's `destructure-array-1.rs` and
`move-ref-patterns-dynamic-semantics.rs` failed on it.

## Decision

**What's left of an array some of whose items moved is dropped as
rustc's `open_drop_for_array` drops it: each item moved by its own path,
as its own flag says, and each run of the items between, a subslice, one
loop, as the array's flag says; the last first, each run in order.**

```js
function middle(items) {
  const m = items[2];
  for (const item of items.slice(3, 6)) {
    noisyDrop_drop(item);
  }
  for (const item of items.slice(0, 2)) {
    noisyDrop_drop(item);
  }
  return m;
}
```

- **The last first**: rustc builds its ladder from the reversed list of
  pieces, so `[1, 2, _, 4, 5, 6]` drops 4, 5, 6 then 1, 2. Native Rust
  does, so rust-js does.
- **A subslice moved**, `[a, rest @ ..]`, is each of its items: rustc's
  move paths have an array's items, never a subslice.
- **A drop flag is a fresh variable set to a constant**, `let a$live =
  true;`, so it goes before a pattern's parts, and a parameter's parts are
  read after it: `function pick([a, b, c, d], i)`, `const [a, , c] =
  items;`, where the flags would have left `_1` and a part a line.

## Why

- **It's the program Rust runs**: the corpus's `array_partial_moves.rs`
  moves items out of arrays as parameters, as `let` patterns, as a rest,
  in one branch with the whole array moved in the other, and with two
  flagged items in one pattern, each with native Rust; both rustc tests
  pass.
- **One way**: an array's drop is the struct's ladder's, by rustc's
  rules, not a second elaboration.

## Consequences

- A run of more than one item is a loop over `slice`, a copy of its part
  of the array, which only the drop reads.
