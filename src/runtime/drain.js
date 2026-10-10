
function $drain(items, start, end = items.length) {
  $checkRange(start, end, items.length);
  return items.splice(start, end - start);
}
