
function $slice(items, start, end = items.length) {
  if (start > end || end > items.length) $sliceIndexFail(start, end, items.length);
  return items.slice(start, end);
}

// `v.extend_from_within(start..end)`'s copy of those items, checked as
// `slice::range` checks them.
function $copyRange(items, start, end = items.length) {
  $checkRange(start, end, items.length);
  return items.slice(start, end);
}
