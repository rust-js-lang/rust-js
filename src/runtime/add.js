// A set's `insert(x)`: whether `x` went in; of items with a destructor, `x`
// dropped by `drop` where one equal's there, which it keeps (ADR 0467).
function $add(set, item, drop) {
  const added = !set.has(item);
  set.add(item);
  if (!added) drop?.(item);
  return added;
}
