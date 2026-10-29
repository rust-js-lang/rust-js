
// `it.skip_while(skip)`: its items from the first `skip` doesn't hold of;
// a `None` of its source while it skips ends what's stepped now, as Rust's
// `find` gives it. Each step is its source's `find`, a `try_fold`, as
// std's is; its `try_fold` and its `fold`, its first step, then its
// source's.
function $lazySkipWhile(items, skip) {
  const source = $stepsOf(items);
  let skipping = true;
  const skipped = new $Steps(
    () => {
      let item;
      const found = $lent(source).tryFold(
        false,
        (_, x) => (!skipping || !skip(x)) && ((skipping = false), (item = x), true),
        (found) => found,
      );
      return found ? { done: false, value: item } : { done: true, value: undefined };
    },
    () => source.return?.(),
  );
  // Where its first step's `skip` throws, a panic, what it's given dropped
  // by `unwind`, as std's `fold(self, init)` drops its own (#423).
  const first = (init, unwind) => {
    if (unwind === undefined) return skipped.next();
    try {
      return skipped.next();
    } catch (error) {
      unwind(init);
      throw error;
    }
  };
  skipped.tryFold = (init, f, broke, unwind) => {
    let folded = init;
    if (skipping) {
      const step = first(folded, unwind);
      if (step.done) return folded;
      folded = f(folded, step.value);
      if (broke(folded)) return folded;
    }
    return $lent(source).tryFold(folded, f, broke, unwind);
  };
  // Its `fold`, std's: its first step, then its source's own fold, the
  // crate's where it writes one (#214); then closed, as `fold(self)` drops
  // what it has left at its end, though its first step found none (#242).
  // Closed though it throws, a panic (#423).
  skipped.fold = (init, f, unwind) =>
    $consumed(skipped, () => {
      let folded = init;
      if (skipping) {
        const step = first(folded, unwind);
        if (step.done) {
          skipped.return();
          return folded;
        }
        folded = f(folded, step.value);
      }
      return source.fold !== undefined ? source.fold(folded, f, unwind) : source.reduce(f, folded);
    });
  skipped.from = [source];
  return skipped;
}
