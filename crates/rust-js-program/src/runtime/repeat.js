
// `vec![item; count]`: `count - 1` clones of `item`, then `item` itself, as
// std's `from_elem` makes them, room for them of `size` bytes each asked
// first (ADR 0774); of none, or of no room, `item` dropped by `drop`, where
// it has a destructor. As a clone unwinds, `item`, then the clones made
// already, in order, as std's `extend_with` drops what it was given before
// `from_elem` drops its `Vec` (#898).
function $repeat(item, count, size, clone, drop) {
  $unwinds(
    () => $roomFor(count, size),
    () => drop?.(item),
  );
  const result = [];
  if (count > 0) {
    $unwinds(
      () => {
        for (let i = 1; i < count; i++) result.push(clone(item));
      },
      () => drop?.(item),
      () => drop && $dropItems(result, drop),
    );
    result.push(item);
  } else {
    drop?.(item);
  }
  return result;
}
