// `m.entry(key).or_insert(value)`: the value put in if the key's missing;
// of a value with a destructor, the one given dropped by `drop` where the
// key's there, as Rust's is (ADR 0321).
function $orInsert(map, key, value, drop) {
  if (!map.has(key)) {
    map.set(key, value);
  } else if (drop !== undefined) {
    drop(value);
  }
  return map.get(key);
}
