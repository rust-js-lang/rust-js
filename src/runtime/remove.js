// A map's `remove(k)`: `Some` of the value it had, boxed where it looks
// like `None` (ADR 0051), or `None`.
function $remove(map, key) {
  const had = map.has(key);
  const old = map.get(key);
  map.delete(key);
  return had ? $some(old) : undefined;
}
