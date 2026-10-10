// std's `slice_index_fail`: why `start..end` isn't a range of a slice of
// `length` items, as Rust says it, its start first.
function $sliceIndexFail(start, end, length) {
  if (start > length) throw new Error(`range start index ${start} out of range for slice of length ${length}`);
  if (end > length) throw new Error(`range end index ${end} out of range for slice of length ${length}`);
  if (start > end) throw new Error(`slice index starts at ${start} but ends at ${end}`);
  throw new Error(`range end index ${end} out of range for slice of length ${length}`);
}

// std's `slice::range`, which `drain`, `splice`, `copy_within` and
// `extend_from_within` check theirs by: its end, then its start.
function $checkRange(start, end, length) {
  if (end > length) $sliceIndexFail(0, end, length);
  if (start > end) $sliceIndexFail(start, end, length);
}
