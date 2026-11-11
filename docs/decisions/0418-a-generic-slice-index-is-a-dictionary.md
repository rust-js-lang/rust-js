# 0418. A slice's index of a type parameter's is a dictionary

Status: Accepted. Extends [0049](0049-traits-and-generics.md), as
[0416](0416-range-bounds-in-generic-code.md) gives `RangeBounds` its.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's indexed, and
each panic, is Rust's.

## Context

A collection of its own items, smallvec's `SmallVec`, indexes as a slice
does with any index a slice takes:

```rust
impl<A: Array, I: SliceIndex<[A::Item]>> Index<I> for SmallVec<A> {
    type Output = I::Output;
    fn index(&self, index: I) -> &I::Output {
        &(**self)[index]
    }
}
```

`v[1]`, `&v[1..3]` and `&mut v[2..]` all call it, `I` a `usize` or a
range. Generic code is compiled once, and `(**self)[index]` of an `I` was
refused: `SliceIndex` had no dictionary.

## Decision

**An `I: SliceIndex<[T]>` is given a dictionary of `index` and
`index_mut`, the runtime's for each kind of index, and `v[i]` of an `I` is
its.**

```js
function smallIndexI_index(small, index, ISliceIndexU32Slice) {
  return ISliceIndexU32Slice.index(index, small.items);
}
smallIndexMutI_index_mut(s, 1, $sliceIndexes.item).value = 20;
smallIndexI_index(s, { start: 1, end: 3 }, $sliceIndexes.range);
```

| Index | `index` | `index_mut` |
|---|---|---|
| `usize`, `$sliceIndexes.item` | its item, checked, `$index` | a handle on its slot, as generic code's `&mut` is |
| a range, `.range`, `.from`, `.to`, `.inclusive`, `.toInclusive`, `.full` | those items, a copy, as a shared subslice is (ADR 0063) | a view of them (ADR 0335), in a box |

- **Which kind a range is, is `recognition::range_bounds`'s**, which
  `RangeBounds`' dictionary asks too; a pair of `Bound`s is refused as an
  index.
- **`index_mut`'s handle is a cell**, as `Std::gives_its_cell` says, so
  its caller writes through it.

## Why

- **It's the program Rust runs**: `test/corpus/slice_index_generic.rs`,
  an `Index<I>` and `IndexMut<I>` of the crate's, indexed by a `usize`,
  each kind of range, written through, sorted and grown, prints what
  native Rust does.
- **One dictionary for each kind, the runtime's**: a call names it,
  `$sliceIndexes.from`, not a function made for it.

## Amendment: an `S: Index<usize>` is its dictionary's (#1420)

rand 0.7.3's `SliceChooseIter` holds `slice: &'a S` of an
`S: Index<usize> + ?Sized` and gives `&self.slice[i]`: refused, "calling
`std::ops::Index::index`", as `Index` wasn't among the traits called
through a dictionary (`recognition::operational`), the crate's own impl's
either. So rand, and quickcheck and itertools' 284 quickcheck tests with it.

`Index` is now one of them, as `Add` is (ADR 0108): `&s[i]` of a type
only bounds know is `SIndexUsize.index(s, i)`, and a bound on one takes
its dictionary. The crate's impl's dictionary is its impl's, exported as
any operator impl's is (the coordinator's decision for the user,
2026-11-12: no rule of `Index`'s own), and std's of a slice, an array, a
`Vec` or a `VecDeque` is `{ index: (items, i) => $index(items, i) }`,
each checked as `v[i]` is, std's panic its message. Of a type known where
it's written, generic code's `Vec<T>` too, `v[i]` stays std's, in place,
and a bound on one, `where Vec<T>: Index<usize, Output = T>`, takes none:
`recognition::indexes_by_dictionary` is the one answer of both. A type
parameter's index stays `SliceIndex`'s, above.

Each crate's `impl Index` now exports its dictionary, one line, `export
const gridIndexUsize = { index: gridIndexUsize_index };`, or a generic
one's `$traitImpl` factory: six corpus snapshots and the queues example gain it, read before
they were blessed; nothing else changed.

Held by the corpus case `index_through_a_generic_bound` against native
Rust in both builds (a slice, an array, a `Vec` of numbers, of strings and
of structs, a `VecDeque`, the crate's own impl, and each kind of panic),
and the mutations `index-not-operational`,
`index-of-a-known-type-by-dictionary` (`lower/recognition/traits.ts`),
`index-of-a-known-type-not-in-place` (`lower/traits/calls.ts`),
`index-bound-of-a-known-type-given` (`lower/traits.ts`),
`std-index-dictionary-missing` and `std-index-dictionary-unchecked`
(`lower/traits/dictionaries.ts`).

