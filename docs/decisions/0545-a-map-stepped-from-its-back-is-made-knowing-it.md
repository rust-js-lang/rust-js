# 0545. A map the crate steps from its back is made knowing it

Status: Accepted. Extends [0455](0455-stepping-from-the-back.md),
[0488](0488-an-extension-bound-iterator-is-an-iterator.md) and [0535](0535-a-lazy-map-steps-from-its-back.md);
accepts more of the [iterators draft](draft/cloned-and-buffering-iterators.md)'s
analysis.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): its items from its
back, and when its function runs, are Rust's.

## Context

itertools' `PadUsing::rfold`, `(self.iter.len()..self.min)
.map(self.filler).rfold(init, &mut f)`, was refused as a library: moving
`init`, which may drop there, writes a drop flag, which first makes what's
pending, the `map` call, a variable, as a call's cleanup reads the flags
as they were where it was made (ADR 0098). A map in a variable has lost
the source its back would step: "a `Map<Range<usize>, F>` as a
`DoubleEndedIterator`". The same of `let mut squares = (1..n).map(f);
squares.next_back()`. The draft asks the iterator's owner to choose before
lowering, not from the JS a value was made of.

## Decision

**`analysis::back_stepped` is the one owner of which `map` and `filter`
types the crate steps from their back: each a `DoubleEndedIterator`
method's `Self`, `next_back`'s, `rfold`'s and the like, of a source not
an array's.** `stepped_from_its_back` is the one answer the lowering asks.

- Such a type knows its back (`knows_its_back`), so where it's made, a
  map is `$mapEnds` and a filter `$filterEnds`, of a range made knowing
  its back too; where it's stepped, it steps as it is, in a variable or
  not.
- A source given as it is gets its back where the map's made, as ADR 0488
  gives it, where its bounds say it has one; where they don't, `either`'s
  forward-only methods, it's made as before.
- `rev()` isn't among them: its own forms reverse an array's or a range's
  items eagerly where that reads better (ADR 0164).

## Why

`map_stepped_from_its_back` is `PadUsing::rfold` in a crate with a type
that may drop, a range's map a variable holds stepped from its back, and
a map of an `ExactSizeIterator` given as it is counted, a range's too,
against native Rust. `extension_map_rfold` holds a map of a type
parameter given as it is in a variable and steps it from its back.

## Consequences

- itertools compiles whole as a library: no refusal left.
- A chain's map stepped from its back is made as it was stepped before,
  `$mapEnds(..)`.
- Mutations see no such type, make it unknowing, give its range as it is,
  and leave out ADR 0488's back or a source's dictionaries.

## Amendment: a map of a `Vec`'s items with a destructor

`vec.into_iter().map(f).rev()` of items with a destructor was refused once
the map was held in a variable: an array's map is left to native forms,
which can't step a source closed to drop its items (ADR 0465), `$closing`.
Such a map, `back_stepped`'s `dropping_items`, is made knowing its back
where it's made, the type `rev()` asks too: `$mapEnds` of its source
closed to drop what it hasn't given, which `$mapEnds` closes as it's
closed, and the `Rev` of it is closed as it's dropped
(`closed_on_drop`). What it never gave is dropped front to back, as Rust
drops it. `test/corpus/rev_lazy_drops.rs` against native Rust; mutations
`rev-of-dropping-map-unasked`, `map-ends-source-unclosed`,
`map-ends-unclosed` and `rev-of-closing-unclosed`.
