// A map's `insert(k, v)`: `Some` of the value it had, boxed where it looks
// like `None` (ADR 0051), or `None`; of keys with a destructor, the key
// given dropped by `dropKey` where one equal's there, which it keeps, as
// Rust's does (ADR 0467): a hashed map's after the value's written, at
// hashbrown's `insert`'s end; a B-tree's (`sorted`) before, as its `entry`
// drops it, the value dropped as that unwinds and the map keeping its own
// (#921). One search, a B-tree's of its keys' own order asking its `cmp`
// as std's does; where that panics, the key given dropped, then the value,
// `dropValue`, as std's `insert` drops them (#911).
function $insert(map, key, value, dropKey, dropValue, sorted) {
  const found = $unwinds(
    () => $found(map, key),
    dropKey && (() => dropKey(key)),
    dropValue && (() => dropValue(value)),
  );
  if (sorted && found.there) $unwinds(() => dropKey(key), dropValue && (() => dropValue(value)));
  $putFound(map, found, key, value);
  if (!sorted && found.there) dropKey?.(key);
  return found.there ? $some(found.value) : undefined;
}
