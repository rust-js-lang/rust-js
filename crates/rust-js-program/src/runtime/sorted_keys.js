// A B-tree set's items in their order, `cmp`'s: of one of its items' own
// order, `$OrdSet`, as it holds them, asking nothing, as std's walk of its
// nodes asks nothing (#911); else sorted.
function $sortedKeys(set, cmp) {
  if (set.found !== undefined) return Array.from(set);
  return $sort(Array.from(set), cmp);
}
