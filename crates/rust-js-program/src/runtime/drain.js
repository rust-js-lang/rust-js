
function $drain(items, start, raisedAt, end = items.length, inclusive) {
  $checkRange(start, end, items.length, raisedAt, inclusive);
  return items.splice(start, end - start);
}
