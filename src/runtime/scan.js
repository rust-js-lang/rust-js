// `it.scan(init, f)` of a JS iterator, lazy as Rust's: each `Some` `f`
// gives, given its state and an item; a `None` ends what's stepped now,
// and a step after steps on, as Rust's `Scan`, which isn't fused, does, of
// an array's (ADR 0517). What it steps is closed as it's closed, where
// Rust drops it, not at a `None`, so a zip's other side drops first. The state's `&mut` is a box of it where it's a `handle`, a
// number's, else the state itself, an object `f` changes in place.
function $lazyScan(items, init, f, handle = true) {
  const state = handle ? { value: init } : init;
  const source = $stepsOf(items);
  const scanning = new $Steps(
    () => {
      const step = source.next();
      const next = step.done ? undefined : f(state, step.value);
      if (next !== undefined) return { done: false, value: next };
      return { done: true, value: undefined };
    },
    () => source.return?.(),
  );
  // Its `try_fold`, its source's, stopped at a `None`.
  scanning.tryFold = (init, g, broke) => {
    let stopped = false;
    const folded = $lent(source).tryFold(
      init,
      (a, x) => {
        const next = f(state, x);
        if (next !== undefined) return g(a, next);
        stopped = true;
        return a;
      },
      (a) => stopped || broke(a),
    );
    return folded;
  };
  // Its `fold`, std's: its `try_fold`, never stopped but by a `None`, then
  // dropped with what it steps (#214).
  scanning.fold = (init, g) => {
    const folded = scanning.tryFold(init, g, () => false);
    scanning.return();
    return folded;
  };
  scanning.from = [source];
  return scanning;
}
