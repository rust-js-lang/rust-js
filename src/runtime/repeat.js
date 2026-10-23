
// `vec![item; count]`: `count - 1` clones of `item`, then `item` itself, as
// std's `from_elem` makes them; of none, `item` dropped by `drop`, where it
// has a destructor.
function $repeat(item, count, clone, drop) {
  const result = [];
  if (count > 0) {
    for (let i = 1; i < count; i++) result.push(clone(item));
    result.push(item);
  } else {
    drop?.(item);
  }
  return result;
}
