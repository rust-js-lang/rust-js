
// `vec![item; count]`: `count - 1` clones of `item`, then `item` itself, as
// std's `from_elem` makes them, room for them of `size` bytes each asked
// first (ADR 0774); of none, or of no room, `item` dropped by `drop`, where
// it has a destructor.
function $repeat(item, count, size, clone, drop) {
  try {
    $roomFor(count, size);
  } catch (panic) {
    drop?.(item);
    throw panic;
  }
  const result = [];
  if (count > 0) {
    for (let i = 1; i < count; i++) result.push(clone(item));
    result.push(item);
  } else {
    drop?.(item);
  }
  return result;
}
