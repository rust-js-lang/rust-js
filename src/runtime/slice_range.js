
// `&v[a..b]`: those items, a part of `v` that says where it is in it
// (#1185), with Rust's panic out of bounds.
function $slice(items, start, end = items.length, inclusive) {
  if (start > end || end > items.length) $sliceIndexFail(start, end, items.length, inclusive);
  return $part(items, start, end);
}

// `v[a..b].to_vec()`, `into()`, a `collect()` of its items copied: a copy
// of those items, owned, an allocation of its own, which no record places
// in `v`, with `&v[a..b]`'s panic out of bounds (#1185).
function $copyOf(items, start, end = items.length, inclusive) {
  if (start > end || end > items.length) $sliceIndexFail(start, end, items.length, inclusive);
  return items.slice(start, end);
}

// `v.extend_from_within(start..end)`'s copy of those items, checked as
// `slice::range` checks them.
function $copyRange(items, start, end = items.length, inclusive) {
  $checkRange(start, end, items.length, inclusive);
  return items.slice(start, end);
}
