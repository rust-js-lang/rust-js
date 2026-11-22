// `v.split_at(mid)` of a slice: copies of the items before `mid` and from
// it, as `&v[..mid]` and `&v[mid..]` are, panicking where it was called,
// `raisedAt`, as Rust's does (#2292).
function $sliceSplitAt(items, mid, raisedAt) {
  if (mid > items.length) throw $panic("mid > len", raisedAt);
  return [$part(items, 0, mid), $part(items, mid)];
}
// `split_at_checked(mid)`: `undefined`, `None`, where `mid` is past the end.
function $checkedSliceSplitAt(items, mid) {
  return mid > items.length ? undefined : [$part(items, 0, mid), $part(items, mid)];
}
