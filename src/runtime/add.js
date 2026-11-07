// A set's `insert(x)`: whether `x` went in; of items with a destructor, `x`
// dropped by `drop` where one equal's there, which it keeps (ADR 0467). One
// search, a B-tree's of its items' own order asking its `cmp` as std's
// does; where that panics, `x` dropped, as std's `insert` drops it (#911).
function $add(set, item, drop) {
  const found = $unwinds(
    () => $found(set, item),
    drop && (() => drop(item)),
  );
  if (found.there) drop?.(item);
  else $putFound(set, found, item);
  return !found.there;
}
