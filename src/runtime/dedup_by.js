// `v.dedup_by(same)`: each item `same` says is the one before it gone; of
// items with a destructor, dropped by `drop` as it's found, before the next
// is compared, as std's are (ADR 0098). Where `same` or a `drop` throws,
// what's left is what it kept then what it hadn't compared yet, as std's
// `FillGapOnDrop` leaves it (#416). Of `cells`, items a `&mut` to is a
// cell, each given `same` as `{ value }`, written back as `same` returns
// or unwinds, so what it wrote is what's kept or dropped (#906).
function $dedupBy(v, same, drop, cells = false) {
  let n = 0;
  let i = 0;
  const compared = () => {
    if (!cells) return same(v[i], v[n - 1]);
    const a = { value: v[i] };
    const b = { value: v[n - 1] };
    try {
      return same(a, b);
    } finally {
      v[i] = a.value;
      v[n - 1] = b.value;
    }
  };
  try {
    while (i < v.length) {
      const duplicate = n !== 0 && compared();
      const item = v[i++];
      if (duplicate) drop?.(item);
      else v[n++] = item;
    }
  } finally {
    const rest = v.length - i;
    if (rest > 0) v.copyWithin(n, i);
    v.length = n + rest;
  }
}
