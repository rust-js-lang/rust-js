
function $sliceEnd(items, start, end = items.length) {
  if (start > end || end > items.length) $sliceIndexFail(start, end, items.length);
  return end;
}
