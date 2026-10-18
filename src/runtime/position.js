
function $position(items, found) {
  let i = 0;
  for (const item of items) {
    if (found(item)) return i;
    i++;
  }
  return undefined;
}

// `rposition(f)` of a slice's items, or a `$iter` of them stepped from its
// front: where the last of what's left that `f` finds is, each tested from
// the back as Rust's is, or `undefined`, `None`. A `$iter` is left stepped
// from its back to it, as Rust leaves it.
function $rposition(items, found) {
  const iter = items.items !== undefined;
  const [array, at, end] = iter ? [items.items, items.at, items.end] : [items, 0, items.length];
  for (let i = end - 1; i >= at; i--) {
    if (found(array[i])) {
      if (iter) items.end = i;
      return i - at;
    }
  }
  if (iter) items.end = at;
  return undefined;
}
