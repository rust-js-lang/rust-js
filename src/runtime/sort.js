// A stable sort by `compare` of items that may be `undefined`, a `None`:
// JS's `sort` moves each `undefined` to the end without asking `compare`,
// where Rust's puts it where its `Ord` says. Their indexes are sorted
// instead, which never are.
function $sort(v, compare) {
  const order = Array.from(v.keys()).sort((i, j) => compare(v[i], v[j]));
  order.map((i) => v[i]).forEach((item, i) => {
    v[i] = item;
  });
  return v;
}
