// A slice's index of a type parameter's, `I: SliceIndex<[T]>`, its
// dictionary (ADR 0418): `usize`'s, its item, checked, and a handle on its
// slot, as generic code's `&mut` is; a range's, those items, a copy as a
// shared subslice is, and a view of them in a box, of each kind of range.
// Each panic raised where generic code indexes, `raisedAt`, as std's impls
// track their caller (#2324).
function $boundsIndex(start, end) {
  return {
    index: (range, items, raisedAt) => $slice(items, start(range), raisedAt, end(range)),
    indexMut: (range, items, raisedAt) => ({ value: $view(items, start(range), raisedAt, end(range)) }),
  };
}

const $sliceIndexes = {
  item: {
    index: (index, items, raisedAt) => $index(items, index, raisedAt),
    indexMut: (index, items, raisedAt) => $ptr(items, $at(items, index, raisedAt)),
  },
  range: $boundsIndex((range) => range.start, (range) => range.end),
  from: $boundsIndex((range) => range.start, () => undefined),
  to: $boundsIndex(() => 0, (range) => range.end),
  inclusive: $boundsIndex((range) => range.start, (range) => range.end + 1),
  toInclusive: $boundsIndex(() => 0, (range) => range.end + 1),
  full: $boundsIndex(() => 0, () => undefined),
};
