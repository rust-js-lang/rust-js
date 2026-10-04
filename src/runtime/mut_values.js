// `m.values_mut()` of a map whose values are numbers or strings: a handle on
// each value, which writes the map, in the order of `entries`, a B-tree's
// sorted (ADR 0152).
function $mutValues(m, entries = m) {
  return Array.from(entries, ([key]) => ({
    get value() {
      return m.get(key);
    },
    set value(item) {
      m.set(key, item);
    },
  }));
}
