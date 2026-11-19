
function $position(items, found) {
  let i = 0;
  const hit = $lent($stepsOf(items)).tryFold(false, (_, item) => found(item) || (i++, false), (hit) => hit);
  return hit ? i : undefined;
}

// `rposition(f)` of a slice's items, or a `$iter` of them stepped from its
// front: where the last of what's left that `f` finds is, each tested from
// the back as Rust's is, or `undefined`, `None`. A `$iter` is left stepped
// from its back to it, as Rust leaves it. Of another JS iterator that knows
// its back and its length, as std's: its length, then folded from its back
// by its own `try_rfold` where it has one, else std's of its `next_back`
// (`$tryRfoldOf`): an `enumerate()`'s asks its source's length once, where
// its `next_back` asks it at each step (#2142).
function $rposition(items, found) {
  if (!Array.isArray(items) && items.items === undefined) {
    let i = items.len();
    const hit = $tryRfoldOf(items, false, (_, item) => (i--, found(item)), (hit) => hit);
    return hit ? i : undefined;
  }
  const iter = items.items !== undefined;
  const [array, at, end] = iter ? [items.items, items.at, items.end] : [items, 0, items.length];
  // Each taken before `f` is asked of it, as `next_back` takes it, so what
  // `f` owns, and drops as it unwinds, a panic, is no longer the `$iter`'s
  // to drop (#738).
  for (let i = end - 1; i >= at; i--) {
    if (iter) items.end = i;
    if (found(array[i])) return i - at;
  }
  if (iter) items.end = at;
  return undefined;
}
