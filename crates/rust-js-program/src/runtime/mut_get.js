// `m.get_mut(k)` of a map whose values are numbers or strings: a handle on
// the key's value, which writes the map, or `undefined`, `None` (ADR 0152).
//
// Of a map of its keys' own order, on its entry, one search (#926).
function $mutGet(m, key) {
  if (m.found !== undefined) {
    const found = m.found(key);
    return found.there ? $entryHandle(found.entry) : undefined;
  }
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
