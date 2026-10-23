// `a.cmp(b)` of two iterators, or `partial_cmp`: each item in turn, as
// Rust's `cmp_by` steps them, the first `Ordering` of two that isn't
// `Equal`, `undefined` where `partial_cmp` of two is `None`; else which
// ended first, `a` stepped before `b` each time.
function $iterCmp(a, b, cmp) {
  const x = a[Symbol.iterator]();
  const y = b[Symbol.iterator]();
  for (;;) {
    const p = x.next();
    if (p.done) return y.next().done ? 0 : -1;
    const q = y.next();
    if (q.done) return 1;
    const order = cmp(p.value, q.value);
    if (order !== 0) return order;
  }
}
