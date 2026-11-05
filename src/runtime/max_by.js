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
// Of a function that drops, what drops with it, `ends`: `$consume`'s
// `reduce`, which drops it before what's left of `items`, as std's
// `reduce(self, f)` does (#734).
function $extremeBy(items, compare, max, boxed, drop, ends) {
  const it = Array.isArray(items) && drop !== undefined ? $closing($iter(items), $dropEach(drop)) : Iterator.from(items);
  let held;
  let holding = false;
  // The one kept dropped where a stage throws, as std's fold closure drops
  // the accumulator it holds, before what's left (#423).
  const unwind = (kept) => {
    holding = false;
    drop(kept);
  };
  const keep = (kept, item) => {
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
  };
  if (ends !== undefined) {
    const kept = $consume(
      it,
      "reduce",
      false,
      (kept, item) => {
        held = kept;
        holding = drop !== undefined;
        return keep(kept, item);
      },
      drop && unwind,
      ends,
    );
    return boxed && kept !== undefined ? $some(kept) : kept;
  }
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
  held = first.value;
  holding = drop !== undefined;
  try {
    held = $foldDropping(it, first.value, keep, drop && unwind);
  } catch (error) {
    if (holding) drop(held);
    throw error;
  }
  return boxed ? $some(held) : held;
}
function $maxBy(items, compare, boxed = false, drop = undefined, ends = undefined) {
  return $extremeBy(items, compare, true, boxed, drop, ends);
}
function $minBy(items, compare, boxed = false, drop = undefined, ends = undefined) {
  return $extremeBy(items, compare, false, boxed, drop, ends);
}
// `max_by_key(key)` and `min_by_key(key)`: each item's key made once, as it
// comes, as std pairs them, and the keys compared by `compare`.
// Where `key` throws, the item it was asked of dropped, as std's pairing
// closure holds it (#423).
function $extremeByKey(items, key, compare, max, boxed, drop, ends) {
  const owned = Array.isArray(items) && drop !== undefined ? $closing($iter(items), $dropEach(drop)) : items;
  // Its first pair made, so the `Map` std pairs them by is folded, which
  // moves `key` into its source's fold, dropped before what's left; till
  // then, where `reduce`'s first `next` ends or throws, the `Map` is
  // dropped whole, its source, then `key` (#734).
  let folding = false;
  let after = false;
  const keyed = Iterator.from(owned).map((item) => {
    let pair;
    try {
      pair = [key(item), item];
    } catch (error) {
      drop?.(item);
      throw error;
    }
    folding = true;
    return pair;
  });
  const dropped =
    ends &&
    (() => {
      if (folding) ends();
      else after = true;
    });
  let pair;
  try {
    pair = $extremeBy(keyed, (a, b) => compare(a[0], b[0]), max, false, drop && ((pair) => drop(pair[1])), dropped);
  } finally {
    if (after) ends();
  }
  return pair === undefined ? undefined : boxed ? $some(pair[1]) : pair[1];
}
function $maxByKey(items, key, compare, boxed = false, drop = undefined, ends = undefined) {
  return $extremeByKey(items, key, compare, true, boxed, drop, ends);
}
function $minByKey(items, key, compare, boxed = false, drop = undefined, ends = undefined) {
  return $extremeByKey(items, key, compare, false, boxed, drop, ends);
}
