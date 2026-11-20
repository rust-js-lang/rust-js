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
// (#870). What it steps it holds, each fresh clone too, so it's dropped
// where the `Cycle` drops it, never as it's done: a clone first, then the
// one it replaces, as `self.iter = self.orig.clone()` (#2160).
function $cycleFrom(orig, iter, clone, next, boxed, drop) {
  iter.hold?.();
  // `self.iter = self.orig.clone()`: the one it steps dropped, then the
  // fresh one in its place, though that drop throws, a panic, as Rust's
  // assignment writes it as it unwinds, so what's closed then is the fresh
  // one, not the one dropped again (#1585).
  const replace = () => {
    const fresh = clone(cycled.orig);
    fresh.hold?.();
    $unwinds(
      () => drop?.(cycled.iter),
      () => {
        cycled.iter = fresh;
      },
    );
    cycled.iter = fresh;
  };
  // What it steps, as std's `Cycle` asks its `I`: the JS iterator it is,
  // with its own steps, or one its `next` steps, a range's or the crate's
  // own, `$given`'s, whose others are std's defaults over it (#2192).
  const stepped = () => $given(cycled.iter, { next }, boxed);
  // Its `try_fold`, of what it steps: its own, else std's default.
  const folded = (init, f, broke, unwind) => {
    const it = stepped();
    return (it.tryFold ?? $Steps.prototype.tryFold).call(it, init, f, broke, unwind);
  };
  const cycled = new $Steps(
    () => {
      if (cycled.$state === "closed") return { done: true, value: undefined };
      let item = next(cycled.iter);
      if (item === undefined) {
        replace();
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
  // Std's `Cycle::try_fold`: what it steps folded to its end by its own
  // `try_fold`, a `Skip`'s asking `nth(n - 1)`; then a fresh clone in its
  // place, folded whole, which, where it gave nothing, ends it, an empty
  // cycle; else a fresh one each round, without end. Each is given `&mut f`,
  // so what drops with `f`, `ends`, is its caller's; where a clone throws,
  // a panic, `ends`, then the accumulator, by `unwind`, as its frame drops
  // its parameters, `f` then `acc` (#2192).
  const tryFold = (init, f, broke, unwind, ends) => {
    if (cycled.$state === "closed") return init;
    let acc = folded(init, f, broke, unwind);
    if (broke(acc)) return acc;
    $stepHolding(replace, acc, unwind, ends);
    let empty = true;
    const seen = (a, x) => {
      empty = false;
      return f(a, x);
    };
    acc = folded(acc, seen, broke, unwind);
    if (broke(acc) || empty) return acc;
    for (;;) {
      $stepHolding(replace, acc, unwind, ends);
      acc = folded(acc, f, broke, unwind);
      if (broke(acc)) return acc;
    }
  };
  // Std's `Cycle::advance_by(n)`: what it steps passed over by its own
  // `advance_by`; where that's short, a fresh clone in its place passed
  // over by what's left, until none is, or a fresh one passes over none,
  // an empty cycle: how many there weren't (#2192).
  const advanceBy = (n, dropItem) => {
    if (cycled.$state === "closed") return n;
    let left = $advanceByOf(stepped(), n, dropItem);
    while (left > 0) {
      replace();
      const short = $advanceByOf(stepped(), left, dropItem);
      if (short === left) return short;
      left = short;
    }
    return 0;
  };
  $forward(cycled, iter, { tryFold, advanceBy });
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
