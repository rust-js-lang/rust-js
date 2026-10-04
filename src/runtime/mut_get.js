// `m.get_mut(k)` of a map whose values are numbers or strings: a handle on
// the key's value, which writes the map, or `undefined`, `None` (ADR 0152).
function $mutGet(m, key) {
  if (!m.has(key)) return undefined;
  return {
    get value() {
      return m.get(key);
    },
    set value(item) {
      m.set(key, item);
    },
  };
}
