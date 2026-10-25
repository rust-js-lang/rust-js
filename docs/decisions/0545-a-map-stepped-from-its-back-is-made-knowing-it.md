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

## Amendment: a flatten from its back

`rev()`, `next_back()` and the like of a `flatten()`, or of a `flat_map(f)`
whose `f` gives an array or a `Vec`, were refused (#165, the bug hunt). A
flatten the crate steps from its back is made knowing it where it's made,
as a map is: `$flattenEnds(src, each)`, which keeps a front and a back
inner list, steps the source from the end it's stepped from, and, the
source done, gives the other end's, as std's `FlattenCompat` does; `f` is
run as each item is reached. A `flat_map` whose `f` gives a lazy iterator
is refused still: its items would have to be made knowing their backs.
Held by `test/corpus/flatten_from_its_back.rs`.

## Amendment: what a function gives behind an `impl DoubleEndedIterator`

A map, a filter or a chain of a slice's items a function gave behind an
`impl DoubleEndedIterator` was refused where its caller stepped it from
its back (#184). The analysis left out a map or a filter of an array's
items, whose own forms step it in place, but the caller has only the
call: no form of its own steps it.

- **The analysis records each adapter a type given behind an `impl` is
  made of** (`given`, where the type asked isn't the one it reveals), an
  array's items' too, and a chain of them, from a call that asks it to be
  double-ended or of a `DoubleEndedIterator` method. A closure's type is
  its body's alone, so what's made in place elsewhere is as it was.
- **Where it's made, it's made knowing its back**: a map `$mapEnds`, a
  filter `$filterEnds`, a chain `$chainEnds`, asked by `knows_its_back`,
  which asks what an `impl` hides, and by `knows_its_length`, which knows a
  map of an array's items made so knows its length too.
- **What drops is decided of the type as it's written**: revealing a
  call's output where it's lowered dropped a consumed iterator early,
  `drops_of_consumed_iterators`'s, so only `knows_its_back` and
  `knows_its_length` reveal it.

ADR 0262's case: C, an error where Rust is fine. Held by
`test/corpus/returned_impl_ends.rs` and the mutations
`given-impl-not-back-stepped`, `given-chain-not-back-stepped`,
`knows-back-of-impl-hidden`, `mapped-array-length-unknown` and
`given-chain-unmade`.

## Amendment: what a map or a filter stepped from its back steps

A map or a filter stepped from its back whose source is another adapter
was accepted and threw `it.back is not a function` (#218):
`[1, 2, 3].iter().map(f).map(g).next_back()` was `$mapEnds` of JS's own
`.map`, which has no back; `(0..9).skip(2).map(f)`'s was `$lazyRange(2, 9)`.
`back_stepped` marked the adapter stepped alone, and `back_kept` took its
source as it was. Now:

- **What it steps is marked too**: each map or filter that a marked one's
  source chain holds, through what steps from its back where its source
  does (`back_stepped_source`), is made knowing its back where it's made,
  held in a variable or not. A `filter_map` held so has no such form; it
  stays unmarked and is refused, not stepped as a JS iterator with no
  back.
- **A source made here is made knowing its back where it's stepped**:
  `back_kept`'s map and filter of a source without a back, a `skip`, a
  `take`, a `step_by`, an `enumerate`, a `chain` or a `fuse` made in
  place, give it to `as_double_ended`, which makes it knowing its back or
  refuses it.

ADR 0262's case: C, an error where Rust is fine, of what isn't made yet;
nothing is approximated. Of 1,100 generated programs pairing adapters
under a back-stepping consumer, none throws: 32 more agree and 44 are
refused, an adapter of the crate's own iterator that steps from its back
(#184). Held by `test/corpus/back_stepped_adapter_sources.rs` and
`held_filter_map_back_refused.rs`, and five mutations. The readability
baseline's `$iter` share grows by one in 10,000 lines: the corpus case's
thirteen arrays stepped from their back are each `$iter(v)`, the source
`$mapEnds` and `$filterEnds` step from either end.
