// `<[T; N]>::map(f)`: each item mapped in turn, as std's is. A panic
// part-way drops, as it unwinds, what was made, then the items not yet
// mapped, as std's guard of its output and its `IntoIter` of what's left
// do, by `$unwinds`; the item being mapped is the closure's, which drops it
// (#1586).
function $arrayMap(items, f, dropMade, dropLeft) {
  const made = [];
  let at = 0;
  $unwinds(
    () => {
      for (; at < items.length; at++) made.push(f(items[at]));
    },
    dropMade && (() => {
      for (const item of made) dropMade(item);
    }),
    dropLeft && (() => {
      for (let i = at + 1; i < items.length; i++) dropLeft(items[i]);
    }),
  );
  return made;
}
