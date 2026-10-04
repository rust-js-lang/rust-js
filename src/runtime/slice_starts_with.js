// `v.starts_with(prefix)`, or `ends_with` (`end`), of a slice whose items `==`
// compares by value, as JS's `===` does: numbers, strings and `bool`s.
function $sliceStartsWith(items, prefix, end = false) {
  const at = end ? items.length - prefix.length : 0;
  return at >= 0 && prefix.length <= items.length && prefix.every((item, i) => item === items[at + i]);
}
