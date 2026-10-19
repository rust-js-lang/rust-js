// `m.entry(key).or_insert_with(make)`: what `make` makes put in if the key's
// missing; where it's there, the key given dropped by `dropKey` (ADR 0467).
function $orInsertWith(map, key, make, dropKey) {
  if (!map.has(key)) {
    map.set(key, make());
  } else {
    dropKey?.(key);
  }
  return map.get(key);
}
