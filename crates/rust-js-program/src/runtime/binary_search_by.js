// `items.binary_search_by(f)`, `f` giving each item's `Ordering` to what's
// sought: `Ok` of where it is, or `Err` of where it would go. The items `f`
// is given are Rust's, in Rust's order: halving, then the last one left.
function $binarySearchBy(items, f) {
  let size = items.length;
  if (size === 0) {
    return { TAG: "Err", _0: 0 };
  }
  let base = 0;
  while (size > 1) {
    const half = size >>> 1;
    const mid = base + half;
    if (f(items[mid]) !== 1) {
      base = mid;
    }
    size -= half;
  }
  const order = f(items[base]);
  return order === 0 ? { TAG: "Ok", _0: base } : { TAG: "Err", _0: base + (order === -1 ? 1 : 0) };
}
