// `items.into_iter()` of a generic `IntoIterator`, as its caller gives it:
// of an array, a `$iter` of it, std's iterator over an array as concrete
// code has it, which knows where it is; else JS's iterator of it, a map's,
// a set's, or one already (ADRs 0061, 0475).
function $intoIter(items) {
  return Array.isArray(items) ? $iter(items) : Iterator.from(items);
}
