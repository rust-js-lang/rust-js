// `v.split_at(mid)` of a slice: copies of the items before `mid` and from
// it, as `&v[..mid]` and `&v[mid..]` are, panicking as Rust's does, or,
// `split_at_checked` (`checked`), `undefined`, `None`.
function $sliceSplitAt(items, mid, checked = false) {
  if (mid > items.length) {
    if (checked) return undefined;
    throw new Error("mid > len");
  }
  return [items.slice(0, mid), items.slice(mid)];
}
