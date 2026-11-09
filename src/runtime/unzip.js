// `unzip()` of `pairs`, an array or an iterator of pairs: each side's
// items, put in as each pair comes, by std's `fold`, which an iterator
// rust-js makes routes through the crate's own (#162). Where a step throws,
// a panic, what's left of `pairs` closed by its fold, then what each side
// collected dropped, `dropA`'s then `dropB`'s, as std's `(A, B)` drops its
// fields as it unwinds (#1150).
function $unzip(pairs, dropA, dropB) {
  const a = [];
  const b = [];
  const put = ([x, y]) => {
    a.push(x);
    b.push(y);
  };
  $unwinds(
    () => (typeof pairs.fold === "function" ? pairs.fold(undefined, (_, pair) => put(pair)) : $eachOf(pairs, put)),
    dropA && (() => $dropItems(a, dropA)),
    dropB && (() => $dropItems(b, dropB)),
  );
  return [a, b];
}
