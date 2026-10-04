// `v.get(start..end)` of a slice: a copy of the items between, as `&v[..]`
// is, or `undefined`, `None`, where that would panic.
function $sliceGet(items, start = 0, end = items.length) {
  return start > end || end > items.length ? undefined : items.slice(start, end);
}
