// `m.entry(key).or_insert(value)`: the value put in if the key's missing;
// where the key's there, the key given dropped by `dropKey`, then the value
// by `drop`, of ones with a destructor, as Rust's are (ADRs 0321, 0467).
function $orInsert(map, key, value, drop, dropKey) {
  if (!map.has(key)) {
    map.set(key, value);
  } else {
    dropKey?.(key);
    drop?.(value);
  }
  return map.get(key);
}
