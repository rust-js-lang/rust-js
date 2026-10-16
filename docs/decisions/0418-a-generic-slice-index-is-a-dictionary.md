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
