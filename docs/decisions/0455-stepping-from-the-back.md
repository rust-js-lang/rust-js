# 0455. `nth_back`, `rfold` and `rfind` step from the back

Status: Accepted. Extends [0429](0429-a-double-ended-iterator-knows-its-back.md)
and [0098](0098-destructors.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same items, in
the same order, dropped where std drops them.

## Context

A double-ended iterator, an array's, a range's, the crate's own or a
generic one, is a JS iterator that knows its back, which `next_back`
steps, `$nextBack(it)` (ADR 0429). But `nth_back`, `rfold` and `rfind`,
which either's `Either` and itertools forward to what they hold, were
refused, of any iterator.

## Decision

**`it.nth_back(n)`, `it.rfold(init, f)` and `it.rfind(p)` are
`$nthBack(it, n)`, `$foldBack(it, init, f)` and `$findBack(it, p)`, which
step `it` from its back as `next_back` does**: the iterator itself where
it has one, as `next_back`'s, and one made as a double-ended iterator is
otherwise, a range's of its bounds.

1. **Of items that could look like `None`**, `nth_back`'s and `rfind`'s
   found item is its `Some`, `$some`, given to the helper (ADR 0051).
2. **Of items with a destructor**, what `nth_back` and `rfind` pass over
   is dropped as it's passed, the item's drop given to the helper, as
   std's are; `rfold` gives each to its function, which drops it. A
   generic one's item of an associated type, which has no drop to give,
   is refused, as forward `nth` is.
3. **`find_map` of items with a destructor**, which gives each to its
   function, isn't refused, as `filter_map`'s isn't (ADR 0098): one
   answer, `moves_each_on`, which recognition asks of a method's name too.

## Why

- **It's std's iterator**: either and itertools forward each method to
  what they hold, and code that walks a list from its end calls them.
- **One way to step from the back**: `it.back()`, which `next_back`
  steps, the same for each kind of iterator.
- **It's tested**: corpus cases step a generic iterator, a range, an
  array's, through a `&mut` and by value, of numbers, of `Option`s, and
  of items with a destructor, against native Rust; a mutation of each
  part is caught.

## Amendment: a map of one knows its back

**A map of an iterator that knows its back knows its back**, as Rust's
`Map` is double-ended where its source is: `$mapEnds(it, f)`, which steps
its source from either end, `f` of each item, `knows_its_back`'s answer,
so a generic one's `it.map(f).rfind(p)` is stepped from its back, as
either's `Either` steps the map it wraps. A map made in place of an
array's items, `v.iter().map(f)`, is made one where it's stepped from its
back. `by_ref()` of one that knows its back is itself, its back kept: a
`$iter` or a `$iterator`, which closing doesn't end, not lent.

## Amendment: a map that knows its back folds by its source's fold (2026-11-04, rd762, rd766)

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change, so
Rust's behavior is kept. `$mapEnds` had std's `Map::rfold` and
`try_rfold`, its source's, but no `fold` or `try_fold`, so it folded by its
`next`: a generic `fold` of the crate's iterator's `map(..)`, given an
`ExactSizeIterator`, never called the crate's own `fold` (#621, #625). It
has them now, its source's, each passing the accumulator's drop on, as
`$Steps`' `map` does. Held by `adapters_over_own_next_that_panics.rs`,
`own_steps_behind_adapters_unwind.rs` and the mutation
`map-ends-fold-not-its-sources`; ADR 0465's amendment of the same date has
the rest.

## Amendment: a map that gives each item as it is (2026-11-11, #1342)

Case ([0262](0262-when-rust-and-js-disagree.md)): Rust's behavior was
refused, so it's kept now.

A map whose function gives each item as it is in JS, `map(Some)` of what
can't look like `None`, is left out of its JS, which is its source's
(`mir/iter.rs`): `.map((value) => value)` reads nothing. Beneath what
steps it from its back or counts it, a filter's, an enumeration's, a
`flat_map`'s or a flatten's `rfold`, what makes that knowing its back found
no `src.map(f)` to make `$mapEnds`, only the source's value under the
map's type, and refused it: "a `Map<Range<u32>, ..>` as a
`DoubleEndedIterator`, which rust-js can't step from its back". #1326 had
turned the flatten's `TypeError` into that refusal and named #1342 in it.

The map's maker says now what it left out: its source's value is marked
`Known::IterMappedAsItIs` (`iterators/value.rs`' `mapped_as_it_is`), and
`as_double_ended` and `as_exact_size` make the map knowing its back or its
length by making its source so, as the map was given it,
`source_mapped_as_it_is`, without the marks its maker gave the map, made
in place. The JS stays as it reads, `$filterEnds($iterator(range, ..),
..)`, no map. A map's lowering is `mir/iter.rs`' `mapped` now, out of
`mir_iter_method`, whose dispatcher cap goes down to 231. A flatten of `Option`s, made a filter of the `Some`s, steps
its source from its back as a filter does (`back_stepped.rs`), so a map a
variable holds beneath it is made knowing its back too. The refusal's
words for #1342 go with it. A map a variable holds beneath an enumeration
stepped from its back is refused as it was, of any function: its length
isn't asked where it's made, which is apart.

`test/corpus/as_is_map_stepped_from_its_back.rs` (a filter, an enumeration,
a `flat_map` and a flatten of a variable's map, from the back, by `rfold`,
`rfind`, `next_back` and `rev()`) runs against native Rust in both builds,
and the conformance table's rows of `(1..6).map(|x| Some(x)).flatten()`
stepped from its back, which #1326 left refused, run as Rust does.
Mutations: `as-is-map-unsaid` (`lower/mir/iter.ts`),
`as-is-map-counted-as-made` and `as-is-map-backed-as-made`
(`lower/iterators/knowing.ts`), `as-is-map-source-kept-in-place`
(`lower/iterators/value.ts`) and `flattened-options-source-unstepped`
(`lower/analysis/back_stepped.ts`); six refitted, and
`as-is-map-refusal-unnamed` taken off with the words it held.
