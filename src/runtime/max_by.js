// `max_by(compare)` and `min_by(compare)` of `items`, an array or an
// iterator: std's `reduce`, its first item, then its fold of the rest, which
// an iterator rust-js makes routes through the crate's own `fold` (#162),
// `compare(kept, item)` as each comes (#179), the later kept where they're
// equal for `max`, the earlier for `min`; each passed over given to `drop`
// as it's passed, of items with a destructor (ADR 0098); boxed where it may
// look like `None` (ADR 0051).
function $extremeBy(items, compare, max, boxed, drop) {
  const it = Iterator.from(items);
  const first = it.next();
  if (first.done) return undefined;
  const kept = it.reduce((kept, item) => {
    if ((compare(kept, item) > 0) === max) {
      drop?.(item);
      return kept;
    }
    drop?.(kept);
    return item;
  }, first.value);
  return boxed ? $some(kept) : kept;
}
function $maxBy(items, compare, boxed = false, drop = undefined) {
  return $extremeBy(items, compare, true, boxed, drop);
}
function $minBy(items, compare, boxed = false, drop = undefined) {
  return $extremeBy(items, compare, false, boxed, drop);
}
// `max_by_key(key)` and `min_by_key(key)`: each item's key made once, as it
// comes, as std pairs them, and the keys compared by `compare`.
function $extremeByKey(items, key, compare, max, boxed, drop) {
  const keyed = Iterator.from(items).map((item) => [key(item), item]);
  const pair = $extremeBy(keyed, (a, b) => compare(a[0], b[0]), max, false, drop && ((pair) => drop(pair[1])));
  return pair === undefined ? undefined : boxed ? $some(pair[1]) : pair[1];
}
function $maxByKey(items, key, compare, boxed = false, drop = undefined) {
  return $extremeByKey(items, key, compare, true, boxed, drop);
}
function $minByKey(items, key, compare, boxed = false, drop = undefined) {
  return $extremeByKey(items, key, compare, false, boxed, drop);
}
