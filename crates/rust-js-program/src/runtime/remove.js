// A map's `remove(k)`: `Some` of the value it had, boxed where it looks
// like `None` (ADR 0051), or `None`; of keys with a destructor, the key it
// held dropped by `dropKey` (ADR 0467). One search, a B-tree's of its keys'
// own order asking its `cmp` as std's does (#911).
function $remove(map, key, dropKey) {
  const found = $found(map, key);
  $deleteFound(map, found, key);
  if (found.there) dropKey?.(found.key);
  return found.there ? $some(found.value) : undefined;
}
