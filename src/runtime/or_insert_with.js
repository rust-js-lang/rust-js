// `m.entry(key).or_insert_with(make)`: what `make` makes put in if the key's
// missing; where it's there, the key given dropped by `dropKey` (ADR 0467),
// then `make`, unrun, by `dropMake` (ADR 0469).
function $orInsertWith(map, key, make, dropKey, dropMake) {
  if (!map.has(key)) {
    map.set(key, make());
  } else {
    dropKey?.(key);
    dropMake?.(make);
  }
  return map.get(key);
}
