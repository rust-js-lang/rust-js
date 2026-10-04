// `v.split_at(mid)` of a slice: copies of the items before `mid` and from
// it, as `&v[..mid]` and `&v[mid..]` are, panicking as Rust's does.
function $sliceSplitAt(items, mid) {
  if (mid > items.length) throw new Error("mid > len");
  return [items.slice(0, mid), items.slice(mid)];
}
