// `v.retain(keep)`: the items `keep` keeps, in order; of items with a
// destructor, each it doesn't dropped by `drop` before the next is asked,
// as std's are (ADR 0098). Where `keep` or a `drop` throws, what's left is
// what it kept then what it hadn't asked of yet, the one it was asking of
// among them, the one it was dropping not, as std's guard puts them back
// (#416).
function $retain(v, keep, drop) {
  let n = 0;
  let i = 0;
  try {
    while (i < v.length) {
      const x = v[i];
      if (keep(x)) {
        v[n++] = x;
        i++;
      } else {
        i++;
        drop?.(x);
      }
    }
  } finally {
    const rest = v.length - i;
    if (rest > 0) v.copyWithin(n, i);
    v.length = n + rest;
  }
}
