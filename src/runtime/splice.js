
function $splice(items, start, end, replacement) {
  $checkRange(start, end, items.length);
  // Pushed, not spread into `splice`'s arguments: as many as there are.
  // What's left of it closed where stepping it throws, a panic (#1126).
  const added = $foldToArray(replacement);
  const tail = items.splice(end);
  const removed = items.splice(start);
  $extend(items, added);
  $extend(items, tail);
  return removed;
}
