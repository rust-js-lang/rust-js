// `m.iter_mut()`, or a `for` over `&mut m`, of a map whose values are numbers
// or strings: each key with a handle on its value, which writes the map, in
// the order of `entries`, a B-tree's sorted (ADR 0152).
// Of a B-tree of its keys' own order, each on its entry, as std's walk asks
// nothing of its keys (#911).
function $mutEntries(m, entries = m) {
  if (m.entryHandles !== undefined) return m.entryHandles(entries);
  return Array.from(entries, ([key]) => [
    key,
    {
      get value() {
        return m.get(key);
      },
      set value(item) {
        m.set(key, item);
      },
    },
  ]);
}
