// `unzip()` of `pairs`, an array or an iterator of pairs: each side's
// items, put in as each pair comes, by std's `fold`, which an iterator
// rust-js makes routes through the crate's own (#162).
function $unzip(pairs) {
  const a = [];
  const b = [];
  Iterator.from(pairs).forEach(([x, y]) => {
    a.push(x);
    b.push(y);
  });
  return [a, b];
}
