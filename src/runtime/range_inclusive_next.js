
// A `RangeInclusive`'s `next()`, `{ start, end }`: its start, moved past,
// while it's at most its end. Past its end, as Rust's, it gives nothing.
function $rangeInclusiveNext(range) {
  return range.start <= range.end ? range.start++ : undefined;
}
