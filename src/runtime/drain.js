
function $drain(items, start, end = items.length, inclusive) {
  $checkRange(start, end, items.length, inclusive);
  return items.splice(start, end - start);
}
