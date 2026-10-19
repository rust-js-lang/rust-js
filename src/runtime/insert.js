// A map's `insert(k, v)`: `Some` of the value it had, boxed where it looks
// like `None` (ADR 0051), or `None`; of keys with a destructor, the key
// given dropped by `dropKey` where one equal's there, which it keeps, as
// Rust's does (ADR 0467).
function $insert(map, key, value, dropKey) {
  const had = map.has(key);
  const old = map.get(key);
  map.set(key, value);
  if (had) dropKey?.(key);
  return had ? $some(old) : undefined;
}
