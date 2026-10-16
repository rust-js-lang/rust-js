// indexmap's `IndexMap` and `IndexSet`: a `Map` and a `Set`, which keep
// what's added in the order it's added, as indexmap's do (ADR 0424). What
// they do by an entry's place, `i`.

// What a map or set keeps `key` as: of one keyed by value, its value's
// string (ADR 0121); of another, itself.
function $keptAs(m, key) {
  return m.keyOf === undefined ? key : m.keyOf(key);
}

// `get_index_of(key)`: where it is, or `None`.
function $indexOf(m, key) {
  const kept = $keptAs(m, key);
  let at = 0;
  for (const k of m.keys()) {
    if ($keptAs(m, k) === kept) return at;
    at++;
  }
  return undefined;
}

// `get_index(i)`, `first()`, `last()`: the entry at `i`, `[key, value]`, or
// a set's item, or `None`.
function $atIndex(m, i) {
  let at = 0;
  for (const entry of m) {
    if (at++ === i) return $some(entry);
  }
  return undefined;
}

// `insert_full(key, value)`: where it is, and the value it had, or `None`.
function $insertFull(m, key, value) {
  const old = $insert(m, key, value);
  return [$indexOf(m, key), old];
}

// A set's `insert_full(item)`: where it is, and whether it's new.
function $addFull(s, item) {
  const added = $add(s, item);
  return [$indexOf(s, item), added];
}

// `swap_remove(key)`, and `remove(key)`: the entry at `key`'s place gone,
// the last in its place, as indexmap's is. Its value, or `None`; of a set,
// whether it was there.
function $swapRemoveKey(m, key) {
  const at = $indexOf(m, key);
  if (at === undefined) return m instanceof Map ? undefined : false;
  const entries = Array.from(m);
  const [removed] = entries.splice(at, 1);
  if (at < entries.length) entries.splice(at, 0, entries.pop());
  $refill(m, entries);
  return m instanceof Map ? $some(removed[1]) : true;
}

// `pop()`: the last entry, `[key, value]`, or a set's item, gone, or `None`.
function $popLast(m) {
  if (m.size === 0) return undefined;
  const entries = Array.from(m);
  const last = entries.pop();
  m.delete(m instanceof Map ? last[0] : last);
  return $some(last);
}

// `sort_keys()`, and a set's `sort()`: in its keys' order, `cmp`'s.
function $sortKeys(m, cmp) {
  const entries = Array.from(m);
  entries.sort(m instanceof Map ? (a, b) => cmp(a[0], b[0]) : cmp);
  $refill(m, entries);
}

// What a map or set holds, `entries`, in their order.
function $refill(m, entries) {
  m.clear();
  for (const entry of entries) {
    if (m instanceof Map) m.set(entry[0], entry[1]);
    else m.add(entry);
  }
}
