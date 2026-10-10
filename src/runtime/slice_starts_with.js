// `v.starts_with(prefix)`, or `ends_with` (`end`): each item `eq` to the
// prefix's, the items' own `==`, or JS's `===` where that's it: numbers,
// strings and `bool`s (ADR 0336).
function $sliceStartsWith(items, prefix, end = false, eq = (a, b) => a === b) {
  const at = end ? items.length - prefix.length : 0;
  return at >= 0 && prefix.length <= items.length && prefix.every((item, i) => eq(items[at + i], item));
}
