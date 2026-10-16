// A slice's index of a type parameter's, `I: SliceIndex<[T]>`, its
// dictionary (ADR 0418): `usize`'s, its item, checked, and a handle on its
// slot, as generic code's `&mut` is; a range's, those items, a copy as a
// shared subslice is, and a view of them in a box, of each kind of range.
function $boundsIndex(start, end) {
  return {
    index: (range, items) => $slice(items, start(range), end(range)),
    index_mut: (range, items) => ({ value: $view(items, start(range), end(range)) }),
  };
}

const $sliceIndexes = {
  item: {
    index: (index, items) => $index(items, index),
    index_mut: (index, items) => $ptr(items, $at(items, index)),
  },
  range: $boundsIndex((range) => range.start, (range) => range.end),
  from: $boundsIndex((range) => range.start, () => undefined),
  to: $boundsIndex(() => 0, (range) => range.end),
  inclusive: $boundsIndex((range) => range.start, (range) => range.end + 1),
  toInclusive: $boundsIndex(() => 0, (range) => range.end + 1),
  full: $boundsIndex(() => 0, () => undefined),
};
