// `max_by(compare)` and `min_by(compare)` of `items`, an array or an
// iterator: std's `reduce`, its first item, then its fold of the rest, which
// an iterator rust-js makes routes through the crate's own `fold` (#162),
// `compare(kept, item)` as each comes (#179), the later kept where they're
// equal for `max`, the earlier for `min`; each passed over given to `drop`
// as it's passed, of items with a destructor (ADR 0098); boxed where it may
// look like `None` (ADR 0051).
//
// Where `compare` throws, a panic, the item it was asked of dropped, then
// the one kept, as std's `max_by(v1, v2)` drops its own as it unwinds,
// then what's left of `items`, an array's too, as its `fold` drops what it
// took; where stepping them throws, a stage's panic, the one kept (#423).
function $extremeBy(items, compare, max, boxed, drop) {
  const it = Array.isArray(items) && drop !== undefined ? $closing($iter(items), $dropEach(drop)) : Iterator.from(items);
  let first;
  // What's left closed where its first step throws, as `reduce` drops
  // what it took.
  try {
    first = it.next();
  } catch (error) {
    it.return?.();
    throw error;
  }
  // None, `reduce`'s `next()?`, which drops it (#242).
  if (first.done) {
    it.return?.();
    return undefined;
  }
  let held = first.value;
  let holding = drop !== undefined;
  try {
    held = it.reduce((kept, item) => {
      let later;
      try {
        later = (compare(kept, item) > 0) !== max;
      } catch (error) {
        holding = false;
        drop?.(item);
        drop?.(kept);
        throw error;
      }
      if (later) {
        drop?.(kept);
        return (held = item);
      }
      drop?.(item);
      return kept;
    }, first.value);
  } catch (error) {
    if (holding) drop(held);
    throw error;
  }
  return boxed ? $some(held) : held;
}
function $maxBy(items, compare, boxed = false, drop = undefined) {
  return $extremeBy(items, compare, true, boxed, drop);
}
function $minBy(items, compare, boxed = false, drop = undefined) {
  return $extremeBy(items, compare, false, boxed, drop);
}
// `max_by_key(key)` and `min_by_key(key)`: each item's key made once, as it
// comes, as std pairs them, and the keys compared by `compare`.
// Where `key` throws, the item it was asked of dropped, as std's pairing
// closure holds it (#423).
function $extremeByKey(items, key, compare, max, boxed, drop) {
  const owned = Array.isArray(items) && drop !== undefined ? $closing($iter(items), $dropEach(drop)) : items;
  const keyed = Iterator.from(owned).map((item) => {
    try {
      return [key(item), item];
    } catch (error) {
      drop?.(item);
      throw error;
    }
  });
  const pair = $extremeBy(keyed, (a, b) => compare(a[0], b[0]), max, false, drop && ((pair) => drop(pair[1])));
  return pair === undefined ? undefined : boxed ? $some(pair[1]) : pair[1];
}
function $maxByKey(items, key, compare, boxed = false, drop = undefined) {
  return $extremeByKey(items, key, compare, true, boxed, drop);
}
function $minByKey(items, key, compare, boxed = false, drop = undefined) {
  return $extremeByKey(items, key, compare, false, boxed, drop);
}
