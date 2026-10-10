
function $splice(items, start, end, replacement) {
  $checkRange(start, end, items.length);
  return items.splice(start, end - start, ...replacement);
}
