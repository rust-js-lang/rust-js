
// `it.skip_while(skip)`: its items from the first `skip` doesn't hold of;
// a `None` of its source while it skips ends what's stepped now, as Rust's
// `find` gives it. Each step is its source's `find`, a `try_fold`, as
// std's is; its `try_fold` and its `fold`, its first step, then its
// source's.
function $lazySkipWhile(items, skip) {
  const source = items[Symbol.iterator]();
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
  skipped.tryFold = (init, f, broke) => {
    let folded = init;
    if (skipping) {
      const step = skipped.next();
      if (step.done) return folded;
      folded = f(folded, step.value);
      if (broke(folded)) return folded;
    }
    return $lent(source).tryFold(folded, f, broke);
  };
  // Its `fold`, std's: its first step, then its source's own fold, the
  // crate's where it writes one (#214).
  skipped.fold = (init, f) => {
    let folded = init;
    if (skipping) {
      const step = skipped.next();
      if (step.done) return folded;
      folded = f(folded, step.value);
    }
    return source.fold !== undefined ? source.fold(folded, f) : source.reduce(f, folded);
  };
  skipped.from = [source];
  return skipped;
}
