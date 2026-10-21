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
  return $cycleFrom(clone(iterator), iterator, clone, next, boxed, drop);
}

// A `Cycle` of what it started with, `orig`, and what it steps, `iter`,
// both kept on it for its `Debug`, its size hint and a clone, where it is,
// which it carries (ADR 0544).
function $cycleFrom(orig, iter, clone, next, boxed, drop) {
  let closed = false;
  const cycled = Iterator.from({
    next() {
      if (closed) return { done: true, value: undefined };
      let item = next(cycled.iter);
      if (item === undefined) {
        const fresh = clone(cycled.orig);
        drop?.(cycled.iter);
        cycled.iter = fresh;
        item = next(cycled.iter);
        if (item === undefined) return { done: true, value: undefined };
      }
      return { done: false, value: boxed ? $someValue(item) : item };
    },
    return() {
      if (!closed) {
        closed = true;
        drop?.(cycled.orig);
        drop?.(cycled.iter);
      }
      return { done: true, value: undefined };
    },
  });
  cycled.orig = orig;
  cycled.iter = iter;
  // Rust's: none of an empty one, else without end.
  cycled.hint = () => {
    const [low, high] = $sizeHint(cycled.orig);
    if (low === 0 && high === 0) return [0, 0];
    return low === 0 ? [0, undefined] : [4294967295, undefined];
  };
  cycled[$cloneKey] = () => $cycleFrom(clone(cycled.orig), clone(cycled.iter), clone, next, boxed, drop);
  return cycled;
}
