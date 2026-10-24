
function $position(items, found) {
  let i = 0;
  const hit = $lent(items[Symbol.iterator]()).tryFold(false, (_, item) => found(item) || (i++, false), (hit) => hit);
  return hit ? i : undefined;
}

// `rposition(f)` of a slice's items, or a `$iter` of them stepped from its
// front: where the last of what's left that `f` finds is, each tested from
// the back as Rust's is, or `undefined`, `None`. A `$iter` is left stepped
// from its back to it, as Rust leaves it. Of another JS iterator that knows
// its back and its length, its length, then each from its back, as std's
// `try_rfold` steps it.
function $rposition(items, found) {
  if (!Array.isArray(items) && items.items === undefined) {
    let i = items.len();
    for (let step = items.back(); !step.done; step = items.back()) {
      i--;
      if (found(step.value)) return i;
    }
    return undefined;
  }
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
