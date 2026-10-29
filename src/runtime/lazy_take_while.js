
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
  // Where `keep` throws, a panic, the accumulator dropped by `unwind`
  // (#423).
  taking.tryFold = (init, f, broke, unwind) => {
    if (ended) return init;
    const kept = unwind === undefined ? (x) => keep(x) : (x, a) => $staged(keep, x, undefined, a, unwind);
    const folded = $lent(source).tryFold(
      init,
      (a, x) => (kept(x, a) ? f(a, x) : ((ended = true), a)),
      (a) => ended || broke(a),
      unwind,
    );
    return folded;
  };
  // Its `fold`, std's: its `try_fold`, never stopped, so its source is
  // folded as std folds it, then dropped with it (#214).
  taking.fold = (init, f, unwind) => {
    const folded = $consumed(taking, () => taking.tryFold(init, f, () => false, unwind));
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
  mapping.tryFold = (init, g, broke, unwind) => {
    let stopped = false;
    const folded = $lent(source).tryFold(
      init,
      (a, x) => {
        const value = unwind === undefined ? f(x) : $staged(f, x, undefined, a, unwind);
        if (present(value)) return g(a, value);
        stopped = true;
        return a;
      },
      (a) => stopped || broke(a),
      unwind,
    );
    return folded;
  };
  // Its `fold`, std's: its `try_fold`, then dropped with what it steps
  // (#214).
  mapping.fold = (init, g, unwind) => {
    const folded = $consumed(mapping, () => mapping.tryFold(init, g, () => false, unwind));
    mapping.return();
    return folded;
  };
  return mapping;
}
