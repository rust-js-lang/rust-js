
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
// one differs or either ends; by `eq`, or `===` of what JS compares so.
function $iterEq(a, b, eq) {
  const x = a[Symbol.iterator]();
  const y = b[Symbol.iterator]();
  for (;;) {
    const p = x.next();
    const q = y.next();
    if (p.done || q.done) return p.done === q.done;
    if (!(eq ? eq(p.value, q.value) : p.value === q.value)) return false;
  }
}
