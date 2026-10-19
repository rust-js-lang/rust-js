// A map's `remove(k)`: `Some` of the value it had, boxed where it looks
// like `None` (ADR 0051), or `None`; of keys with a destructor, the key it
// held dropped by `dropKey` (ADR 0467).
function $remove(map, key, dropKey) {
  const had = map.has(key);
  const old = map.get(key);
  const held = had && dropKey !== undefined ? $storedKey(map, key) : undefined;
  map.delete(key);
  if (had) dropKey?.(held);
  return had ? $some(old) : undefined;
}
