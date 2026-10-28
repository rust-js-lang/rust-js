// `items.into_iter()` of a generic `IntoIterator`, as its caller gives it:
// of an array, a `$iter` of it, std's iterator over an array as concrete
// code has it, which knows where it is; else JS's iterator of it, a map's,
// a set's, or one already (ADRs 0061, 0475): a `$Steps` as it is, not
// stepped by JS's own protocol, which closes it at its end, so a held one
// is its owner's to drop, where Rust drops it (#392).
function $intoIter(items) {
  if (Array.isArray(items)) return $iter(items);
  return items instanceof $Steps ? items : Iterator.from(items);
}
