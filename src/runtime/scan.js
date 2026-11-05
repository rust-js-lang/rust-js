// `it.scan(init, f)` of a JS iterator, lazy as Rust's: each `Some` `f`
// gives, given its state and an item; a `None` ends what's stepped now,
// and a step after steps on, as Rust's `Scan`, which isn't fused, does, of
// an array's (ADR 0517). What it steps is closed as it's closed, where
// Rust drops it, not at a `None`, so a zip's other side drops first. The state's `&mut` is a box of it where it's a `handle`, a
// number's, else the state itself, an object `f` changes in place. Given
// `drop`, what drops its function, then its state, given its state: run
// once it's closed, after what it steps, as `Scan { iter, f, state }` drops
// its fields (#760).
function $lazyScan(items, init, f, handle = true, drop = undefined) {
  const state = handle ? { value: init } : init;
  const source = $stepsOf(items);
  let dropped = drop === undefined;
  const scanning = new $Steps(
    () => {
      const step = source.next();
      const next = step.done ? undefined : f(state, step.value);
      if (next !== undefined) return { done: false, value: next };
      return { done: true, value: undefined };
    },
    () =>
      $closedAfter(
        () => source.return?.(),
        () => {
          if (dropped) return;
          dropped = true;
          drop(handle ? state.value : state);
        },
      ),
  );
  // Its `try_fold`, its source's, stopped at a `None`.
  // Where `f` throws, a panic, the accumulator dropped by `unwind` (#423).
  const tryFold = (init, g, broke, unwind, ends) => {
    let stopped = false;
    const folded = $lent(source).tryFold(
      init,
      (a, x) => {
        const next = unwind === undefined ? f(state, x) : $staged((x) => f(state, x), x, undefined, a, unwind);
        if (next !== undefined) return g(a, next);
        stopped = true;
        return a;
      },
      (a) => stopped || broke(a),
      unwind,
      ends,
    );
    return folded;
  };
  // Its `fold`, std's: its `try_fold`, never stopped but by a `None`, then
  // dropped with what it steps (#214).
  // What drops with `g`, `ends`, first (#734).
  const fold = (init, g, unwind, ends) => {
    const folded = $consumed(scanning, () => scanning.tryFold(init, g, () => false, unwind), ends);
    $closedAfter(ends, () => scanning.return());
    return folded;
  };
  $forward(scanning, source, { tryFold, fold });
  // Std's `Scan::size_hint`, none to its source's upper bound.
  scanning.sizeHint = () => $upperOnly(source);
  scanning.from = [source];
  return scanning;
}
