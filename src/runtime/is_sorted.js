// `is_sorted()` of a slice's or an iterator's items: each at most the next,
// by `<=`, or where that isn't their `PartialOrd`'s, by `cmp`, their
// `Ordering`, `Less` or `Equal`. It stops at the first that isn't, as
// Rust's does, so an iterator's later items aren't made.
function $isSorted(items, cmp) {
  let first = true;
  let last;
  for (const item of items) {
    if (!first) {
      const order = cmp ? cmp(last, item) : undefined;
      if (cmp ? order !== -1 && order !== 0 : !(last <= item)) return false;
    }
    first = false;
    last = item;
  }
  return true;
}
