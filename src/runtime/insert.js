// A map's `insert(k, v)`: `Some` of the value it had, boxed where it looks
// like `None` (ADR 0051), or `None`.
function $insert(map, key, value) {
  const had = map.has(key);
  const old = map.get(key);
  map.set(key, value);
  return had ? $some(old) : undefined;
}
