// `v.dedup_by(same)`: each item `same` says is the one before it gone; of
// items with a destructor, dropped by `drop` before the next is compared,
// as std's are (ADR 0098). Where `same` or a `drop` throws, what's left is
// what it kept then what it hadn't compared yet, as `$retain` leaves it
// (#416).
function $dedupBy(v, same, drop) {
  let n = 0;
  let i = 0;
  try {
    while (i < v.length) {
      const item = v[i];
      if (n === 0 || !same(item, v[n - 1])) {
        v[n++] = item;
        i++;
      } else {
        i++;
        drop?.(item);
      }
    }
  } finally {
    const rest = v.length - i;
    if (rest > 0) v.copyWithin(n, i);
    v.length = n + rest;
  }
}
