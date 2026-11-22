// `lists.join(&sep)` of a slice of lists (ADR 0322): their items in one
// array, with `sep` between each two, or `sep`'s items where it's a slice,
// each cloned by `clone` where a clone is more than the item.
function $joinWith(lists, sep, spread, clone) {
  const out = [];
  const copy = clone ?? ((item) => item);
  lists.forEach((list, i) => {
    if (i > 0) {
      for (const item of spread ? sep : [sep]) out.push(copy(item));
    }
    for (const item of list) out.push(copy(item));
  });
  return out;
}
