// `m.iter_mut()`, or a `for` over `&mut m`, of a map whose values are numbers
// or strings: each key with a handle on its value, which writes the map, in
// the order of `entries`, a B-tree's sorted (ADR 0152).
function $mutEntries(m, entries = m) {
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
