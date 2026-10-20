// `it.cycle()` of std's iterator over an array: what it has left, `items`,
// gone over again and again, a clone of each item it owns that needs one
// (ADR 0181); none at all of one with none left, as Rust's.
function* $cycle(items, clone = (item) => item) {
  if (items.length === 0) return;
  while (true) {
    for (const item of items) yield clone(item);
  }
}

// `it.cycle()` of any other iterator, stepped by `next`, `clone` making a
// clone of it: one kept as it's given, and each time what's stepped ends,
// a fresh clone of that stepped instead, the one it ends dropped then, none
// at all once a fresh one ends at once, as Rust's `Cycle` (ADR 0491).
// Closed, it drops both, the one kept first, as `Cycle`'s fields are. `boxed`: whether `next` gives a `Some` that may look like
// `None` boxed (ADR 0051).
function $cycleOf(iterator, clone, next, boxed = false, drop) {
  const orig = clone(iterator);
  let it = iterator;
  let closed = false;
  return Iterator.from({
    next() {
      if (closed) return { done: true, value: undefined };
      let item = next(it);
      if (item === undefined) {
        const fresh = clone(orig);
        drop?.(it);
        it = fresh;
        item = next(it);
        if (item === undefined) return { done: true, value: undefined };
      }
      return { done: false, value: boxed ? $someValue(item) : item };
    },
    return() {
      if (!closed) {
        closed = true;
        drop?.(orig);
        drop?.(it);
      }
      return { done: true, value: undefined };
    },
  });
}
