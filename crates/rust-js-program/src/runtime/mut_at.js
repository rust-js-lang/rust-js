// `v.get_mut(i)`, `first_mut()` and `last_mut()` of numbers or strings: a
// handle on the item at `i`, from the end if it's negative, or `undefined`,
// `None`, past either end (ADR 0152).
function $mutAt(v, i) {
  const at = i < 0 ? v.length + i : i;
  if (at < 0 || at >= v.length) return undefined;
  return {
    get value() {
      return v[at];
    },
    set value(item) {
      v[at] = item;
    },
  };
}
