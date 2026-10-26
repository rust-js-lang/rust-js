
function $splice(items, start, end, replacement) {
  $checkRange(start, end, items.length);
  // Pushed, not spread into `splice`'s arguments: as many as there are.
  const added = Array.from(replacement);
  const tail = items.splice(end);
  const removed = items.splice(start);
  $extend(items, added);
  $extend(items, tail);
  return removed;
}
