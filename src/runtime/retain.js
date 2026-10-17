// `v.retain(keep)`: the items `keep` keeps, in order; of items with a
// destructor, each it doesn't dropped by `drop` before the next is asked,
// as std's are (ADR 0098).
function $retain(v, keep, drop) {
  let n = 0;
  for (const x of v) {
    if (keep(x)) {
      v[n++] = x;
    } else if (drop !== undefined) {
      drop(x);
    }
  }
  v.length = n;
}
