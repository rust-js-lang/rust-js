
// `it.take_while(keep)`: each item `keep` holds of; the first it doesn't
// ends it for good, closing what it steps, as Rust's flag does, and a
// `None` of its source ends what's stepped now, as Rust's passes it on.
// Its `try_fold`, its source's, stopped where `keep` doesn't hold.
function $lazyTakeWhile(items, keep) {
  const source = items[Symbol.iterator]();
  let ended = false;
  const taking = new $Steps(
    () => {
      if (ended) return { done: true, value: undefined };
      const step = source.next();
      if (step.done || keep(step.value)) return step;
      ended = true;
      source.return?.();
      return { done: true, value: undefined };
    },
    () => source.return?.(),
  );
  taking.tryFold = (init, f, broke) => {
    if (ended) return init;
    const folded = $lent(source).tryFold(init, (a, x) => (keep(x) ? f(a, x) : ((ended = true), a)), (a) => ended || broke(a));
    if (ended) source.return?.();
    return folded;
  };
  // Its `fold`, std's: its `try_fold`, never stopped, so its source is
  // folded as std folds it (#214).
  taking.fold = (init, f) => taking.tryFold(init, f, () => false);
  return taking;
}

// `it.map_while(f)`: each of what `f` gives that `present` keeps; one it
// doesn't ends what's stepped now, closing what it steps, as `$lazyScan`'s
// `None` does, and a step after steps on, as Rust's `MapWhile`, which
// isn't fused, does, where `take_while` stays ended. Its `try_fold`, its
// source's, stopped at what `present` doesn't keep.
function $lazyMapWhile(items, f, present) {
  const source = items[Symbol.iterator]();
  const mapping = new $Steps(
    () => {
      const step = source.next();
      const value = step.done ? undefined : f(step.value);
      if (!step.done && present(value)) return { done: false, value };
      source.return?.();
      return { done: true, value: undefined };
    },
    () => source.return?.(),
  );
  mapping.tryFold = (init, g, broke) => {
    let stopped = false;
    const folded = $lent(source).tryFold(
      init,
      (a, x) => {
        const value = f(x);
        if (present(value)) return g(a, value);
        stopped = true;
        return a;
      },
      (a) => stopped || broke(a),
    );
    if (stopped) source.return?.();
    return folded;
  };
  return mapping;
}
