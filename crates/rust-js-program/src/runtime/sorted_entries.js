// A B-tree's entries in its keys' order, `cmp`'s: of one of its keys' own
// order, `$OrdMap`, as it holds them, asking nothing, as std's walk of
// its nodes asks nothing (#911); else sorted.
function $sortedEntries(map, cmp) {
  if (map.found !== undefined) return Array.from(map);
  return Array.from(map).sort((a, b) => cmp(a[0], b[0]));
}
