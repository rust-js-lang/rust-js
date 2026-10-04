// `v.iter_mut()` of numbers or strings, which JS can't change in place: a
// handle on each item, whose `value` reads and writes it (ADR 0152).
function $mutItems(v) {
  return Array.from(v, (_, i) => ({
    get value() {
      return v[i];
    },
    set value(item) {
      v[i] = item;
    },
  }));
}
