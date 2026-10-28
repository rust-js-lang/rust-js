
// `it.take_while(keep)`: each item `keep` holds of; the first it doesn't
// ends it for good, as Rust's flag does, and a `None` of its source ends
// what's stepped now, as Rust's passes it on. What it steps is closed as
// it's closed, where Rust drops it, not where it ends.
// Its `try_fold`, its source's, stopped where `keep` doesn't hold.
function $lazyTakeWhile(items, keep) {
  const source = $stepsOf(items);
  let ended = false;
  const taking = new $Steps(
    () => {
      if (ended) return { done: true, value: undefined };
      const step = source.next();
      if (step.done || keep(step.value)) return step;
      ended = true;
      return { done: true, value: undefined };
    },
    () => source.return?.(),
  );
  taking.from = [source];
  taking.tryFold = (init, f, broke) => {
    if (ended) return init;
    const folded = $lent(source).tryFold(init, (a, x) => (keep(x) ? f(a, x) : ((ended = true), a)), (a) => ended || broke(a));
    return folded;
  };
  // Its `fold`, std's: its `try_fold`, never stopped, so its source is
  // folded as std folds it, then dropped with it (#214).
  taking.fold = (init, f) => {
    const folded = taking.tryFold(init, f, () => false);
    taking.return();
    return folded;
  };
  return taking;
}

// `it.map_while(f)`: each of what `f` gives that `present` keeps; one it
// doesn't ends what's stepped now, as `$lazyScan`'s `None` does, and a
// step after steps on, as Rust's `MapWhile`, which isn't fused, does, where
// `take_while` stays ended. What it steps is closed as it's closed. Its `try_fold`, its
// source's, stopped at what `present` doesn't keep.
function $lazyMapWhile(items, f, present) {
  const source = $stepsOf(items);
  const mapping = new $Steps(
    () => {
      const step = source.next();
      const value = step.done ? undefined : f(step.value);
      if (!step.done && present(value)) return { done: false, value };
      return { done: true, value: undefined };
    },
    () => source.return?.(),
  );
  mapping.from = [source];
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
    return folded;
  };
  // Its `fold`, std's: its `try_fold`, then dropped with what it steps
  // (#214).
  mapping.fold = (init, g) => {
    const folded = mapping.tryFold(init, g, () => false);
    mapping.return();
    return folded;
  };
  return mapping;
}
