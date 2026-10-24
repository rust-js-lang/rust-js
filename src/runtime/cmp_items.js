
function $cmpItems(a, b, cmp) {
  for (let i = 0; i < a.length && i < b.length; i++) {
    const order = cmp(a[i], b[i]);
    if (order !== 0) {
      return order;
    }
  }
  return $cmp(a.length, b.length);
}
// `a.eq(b)` of two iterators: each item in turn, as Rust steps them, till
// one differs or either ends; by `eq`, or `===` of what JS compares so. Of
// items with a destructor, each compared pair dropped, `b`'s first, and one
// left where the other ended; each closed as it ends, which drops what it
// holds, `b` first, as std's are dropped (ADR 0465). `a` is stepped by its
// `tryFold`, as std's `iter_compare` steps it by `try_for_each`, and `b` by
// its `next`.
function $iterEq(a, b, eq, dropA, dropB) {
  const x = a[Symbol.iterator]();
  const y = b[Symbol.iterator]();
  try {
    let same = true;
    const stopped = $lent(x).tryFold(
      false,
      (_, p) => {
        const q = y.next();
        if (q.done) {
          dropA?.(p);
          same = false;
          return true;
        }
        same = eq ? eq(p, q.value) : p === q.value;
        dropB?.(q.value);
        dropA?.(p);
        return !same;
      },
      (stop) => stop,
    );
    if (stopped) return same;
    const q = y.next();
    if (q.done) return true;
    dropB?.(q.value);
    return false;
  } finally {
    y.return?.();
    x.return?.();
  }
}
