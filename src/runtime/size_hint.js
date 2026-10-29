// `it.size_hint()` of a generic iterator (ADR 0170): exact of one stepping
// through an array, `$iter`'s, which is how an array is given as one, and
// std's `(0, None)` of a lazy one, which may be any length; Rust lets a hint
// be any bounds that hold. Of a value given as it is, its `Iterator`
// dictionary's own, `iterator.size_hint`, where its impl has one (ADR 0411).
function $sizeHint(it, iterator) {
  if (iterator?.sizeHint !== undefined) return iterator.sizeHint(it);
  // A range given as one, `{ next: $rangeNext }`: its own, `$rangeHintOf`'s,
  // exact of numbers, as it knows how many it has left, and of `n..`,
  // `{ next: $rangeFromNext }`, `usize::MAX` and none known (ADR 0918, #400).
  const ranged = iterator === undefined ? undefined : $rangeHintOf(iterator.next);
  if (ranged !== undefined) return ranged(it);
  // An explicit adapter's, its own, of what it steps (ADR 0542).
  if (it.hint !== undefined) return it.hint();
  // What a `$iterator` steps, a JS iterator's, an array's `$iter` given as
  // it is (ADR 0411).
  if (typeof it.of?.next === "function") return $sizeHint(it.of);
  const left = Array.isArray(it.items) ? it.end - it.at : undefined;
  return left === undefined ? [0, undefined] : [left, left];
}
