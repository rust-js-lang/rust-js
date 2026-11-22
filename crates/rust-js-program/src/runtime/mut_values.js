// `m.values_mut()` of a map whose values are numbers or strings: a handle on
// each value, which writes the map, in the order of `entries`, a B-tree's
// sorted (ADR 0152). Each on its entry, `$mutEntries`', of a map whose
// search can be seen, as std's walk asks nothing of its keys (#1362).
function $mutValues(m, entries = m) {
  return $mutEntries(m, entries).map(([, handle]) => handle);
}
