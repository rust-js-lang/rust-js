
// `it.skip_while(skip)`: its items from the first `skip` doesn't hold of;
// a `None` of its source while it skips ends what's stepped now, as Rust's
// `find` gives it. Each step is its source's `find`, a `try_fold`, as
// std's is; its `try_fold` and its `fold`, its first step, then its
// source's. Of items with a destructor, each it skips dropped by `drop`,
// and the one `skip` panics on as it unwinds, as std's `SkipWhile` drops
// both in its `check` closure, whose frame holds no accumulator: `$kept`'s,
// of what `skip` doesn't skip (#423, #817).
function $lazySkipWhile(items, skip, drop) {
  const source = $stepsOf(items);
  let skipping = true;
  const skipped = new $Steps(
    () => {
      let item;
      const found = $lent(source).tryFold(
        false,
        (_, x) => (!skipping || $kept((y) => !skip(y), drop, x)) && ((skipping = false), (item = x), true),
        (found) => found,
      );
      return found ? { done: false, value: item } : { done: true, value: undefined };
    },
    () => source.return?.(),
  );
  // Where its first step's `skip` throws, a panic, what it's given dropped
  // as std's `fold(self, init, fold)` and `try_fold(&mut self, init, fold)`
  // drop their own, which hold nothing else then: what drops with `f`,
  // `ends`, where it's given, then `init`, by `unwind`, the second though
  // the first throws (#423, #768).
  const first = (init, unwind, ends) => $stepHolding(() => skipped.next(), init, unwind, ends);
  const tryFold = (init, f, broke, unwind, ends) => {
    let folded = init;
    if (skipping) {
      const step = first(folded, unwind, ends);
      if (step.done) return folded;
      folded = f(folded, step.value);
      if (broke(folded)) return folded;
    }
    return $lent(source).tryFold(folded, f, broke, unwind, ends);
  };
  // Its `fold`, std's: its first step, then its source's own fold, the
  // crate's where it writes one (#214); then closed, as `fold(self)` drops
  // what it has left at its end, though its first step found none (#242).
  // Closed though it throws, a panic (#423).
  // What drops with `f`, `ends`, its source's fold's, which `SkipWhile::fold`
  // gives `fold` by value, or first where it stops (#734).
  const fold = (init, f, unwind, ends) => {
    return $consumed(
      skipped,
      () => {
        let folded = init;
        if (skipping) {
          const step = first(folded, unwind, ends);
          if (step.done) {
            $closedAfter(ends, () => skipped.return());
            return folded;
          }
          folded = f(folded, step.value);
        }
        return source.fold !== undefined ? source.fold(folded, f, unwind, ends) : source.reduce(f, folded);
      },
      ends,
    );
  };
  $forward(skipped, source, { tryFold, fold });
  // Std's `SkipWhile::size_hint`, none to its source's upper bound, once
  // it's done skipping too (skip_while.rs:65).
  skipped.sizeHint = () => $upperOnly(source);
  skipped.from = [source];
  return skipped;
}
