// `it.cycle()` of std's iterator over an array: what it has left, `items`,
// gone over again and again, a clone of each item it owns that needs one
// (ADR 0181); none at all of one with none left, as Rust's.
function* $cycle(items, clone = (item) => item) {
  if (items.length === 0) return;
  while (true) {
    for (const item of items) yield clone(item);
  }
}
