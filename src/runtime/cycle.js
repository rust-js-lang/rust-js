// `it.cycle()` of std's iterator over an array: what it has left, `items`,
// gone over again and again, a clone of each item it owns that needs one
// (ADR 0181); none at all of one with none left, as Rust's. Its hint
// Rust's, none of no items, else without end (#1179). A `$Steps`, as the
// compiler takes every `cycle()` to be, so a `take(n)` of it is `$taking`,
// whose hint is std's `Take`'s of it, which a `Vec` grows by: JS's own
// `take` of a generator knows none, and grew one by doubling, 8 where std's
// is 7 (#1572).
function $cycle(items, clone = (item) => item) {
  let at = 0;
  const cycled = new $Steps(() => {
    if (items.length === 0) return { done: true, value: undefined };
    const item = items[at];
    at = (at + 1) % items.length;
    return { done: false, value: clone(item) };
  });
  cycled.sizeHint = () => (items.length === 0 ? [0, 0] : [4294967295, undefined]);
  return cycled;
}

// `it.cycle()` of any other iterator, stepped by `next`, `clone` making a
// clone of it: one kept as it's given, and each time what's stepped ends,
// a fresh clone of that stepped instead, the one it ends dropped then, none
// at all once a fresh one ends at once, as Rust's `Cycle` (ADR 0491).
// Closed, it drops both, the one kept first, as `Cycle`'s fields are, the
// second though the first's drop throws, a panic, `$inTurn`'s, as its drop
// glue goes on (#883). `boxed`: whether `next` gives a `Some` that may look
// like `None` boxed (ADR 0051). Where the clone it keeps throws, a panic,
// what it was given is dropped as it unwinds, a local of `Cycle::new`'s
// frame (#1585).
function $cycleOf(iterator, clone, next, boxed = false, drop) {
  const orig = $unwinds(() => clone(iterator), drop && (() => drop(iterator)));
  return $cycleFrom(orig, iterator, clone, next, boxed, drop);
}

// A `Cycle` of what it started with, `orig`, and what it steps, `iter`,
// both kept on it for its `Debug`, its size hint and a clone, where it is,
// which it carries (ADR 0544). A `$Steps`, whose `take` closes it though a
// clone throws, a panic, as Rust drops a `Take` that holds it as it
// unwinds (#708). Its lifecycle `$closing`'s, `$close`'s, never taken
// (#870).
function $cycleFrom(orig, iter, clone, next, boxed, drop) {
  const cycled = new $Steps(
    () => {
      if (cycled.$state === "closed") return { done: true, value: undefined };
      let item = next(cycled.iter);
      if (item === undefined) {
        // `self.iter = self.orig.clone()`: the one it steps dropped, then
        // the fresh one in its place, though that drop throws, a panic, as
        // Rust's assignment writes it as it unwinds, so what's closed then
        // is the fresh one, not the one dropped again (#1585).
        const fresh = clone(cycled.orig);
        $unwinds(
          () => drop?.(cycled.iter),
          () => {
            cycled.iter = fresh;
          },
        );
        cycled.iter = fresh;
        item = next(cycled.iter);
        if (item === undefined) return { done: true, value: undefined };
      }
      return { done: false, value: boxed ? $someValue(item) : item };
    },
    () =>
      $close(cycled, () =>
        $inTurn(
          () => drop?.(cycled.orig),
          () => drop?.(cycled.iter),
        ),
      ),
  );
  cycled.orig = orig;
  cycled.iter = iter;
  // Rust's: none of an empty one, else without end; of its source as its
  // step asks it, a range's by its `next` (#1179).
  cycled.sizeHint = () => {
    const [low, high] = $sizeHint(cycled.orig, { next });
    if (low === 0 && high === 0) return [0, 0];
    return low === 0 ? [0, undefined] : [4294967295, undefined];
  };
  // Its derived `Clone`, each field's in turn: where the clone of what it
  // steps throws, a panic, the clone of what it started with is dropped as
  // it unwinds (#1585).
  cycled[$cloneKey] = () => {
    const orig = clone(cycled.orig);
    const iter = $unwinds(() => clone(cycled.iter), drop && (() => drop(orig)));
    return $cycleFrom(orig, iter, clone, next, boxed, drop);
  };
  return cycled;
}
