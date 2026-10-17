// `v.dedup_by(same)`: each item `same` says is the one before it gone; of
// items with a destructor, dropped by `drop` before the next is compared,
// as std's are (ADR 0098).
function $dedupBy(v, same, drop) {
  let n = 0;
  for (const item of v) {
    if (n === 0 || !same(item, v[n - 1])) {
      v[n++] = item;
    } else if (drop !== undefined) {
      drop(item);
    }
  }
  v.length = n;
}
