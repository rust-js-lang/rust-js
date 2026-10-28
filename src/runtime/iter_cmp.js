// `a.cmp(b)` of two iterators, or `partial_cmp`: each item in turn, as
// Rust's `cmp_by` steps them, the first `Ordering` of two that isn't
// `Equal`, `undefined` where `partial_cmp` of two is `None`; else which
// ended first, `a` stepped before `b` each time: `a` by its `tryFold`, as
// std's `iter_compare` steps it by `try_for_each`, `b` by its `next`. Each
// closed at its end, `b` first, as std drops what it takes (#242).
function $iterCmp(a, b, cmp) {
  const x = $stepsOf(a);
  const y = $stepsOf(b);
  try {
    let order = 0;
    const stopped = $lent(x).tryFold(
      false,
      (_, p) => {
        const q = y.next();
        order = q.done ? 1 : cmp(p, q.value);
        return order !== 0;
      },
      (stop) => stop,
    );
    if (stopped) return order;
    return y.next().done ? 0 : -1;
  } finally {
    y.return?.();
    x.return?.();
  }
}
