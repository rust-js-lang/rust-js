
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
// holds, `b` first, as std's are dropped (ADR 0465).
function $iterEq(a, b, eq, dropA, dropB) {
  const x = a[Symbol.iterator]();
  const y = b[Symbol.iterator]();
  try {
    for (;;) {
      const p = x.next();
      const q = y.next();
      if (p.done || q.done) {
        if (!q.done) dropB?.(q.value);
        if (!p.done) dropA?.(p.value);
        return p.done === q.done;
      }
      const same = eq ? eq(p.value, q.value) : p.value === q.value;
      dropB?.(q.value);
      dropA?.(p.value);
      if (!same) return false;
    }
  } finally {
    y.return?.();
    x.return?.();
  }
}
