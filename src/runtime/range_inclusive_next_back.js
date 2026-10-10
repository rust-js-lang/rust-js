// A `RangeInclusive`'s `next_back()`, `{ start, end }`: its end, moved
// down, while it's at least its start.
function $rangeInclusiveNextBack(range) {
  return range.start <= range.end ? range.end-- : undefined;
}
