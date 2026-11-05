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
  if (it.sizeHint !== undefined) return it.sizeHint();
  // What a `$iterator` steps, a JS iterator's, an array's `$iter` given as
  // it is (ADR 0411).
  if (typeof it.of?.next === "function") return $sizeHint(it.of);
  const left = Array.isArray(it.items) ? it.end - it.at : undefined;
  return left === undefined ? [0, undefined] : [left, left];
}

// Std's hint of an adapter that may give fewer than its source, none at
// all: `(0, upper)`, its source's upper bound, as `Filter`, `FilterMap`,
// `SkipWhile`, `TakeWhile`, `MapWhile` and `Scan` give it (filter.rs:118,
// filter_map.rs:126, skip_while.rs:65, take_while.rs:59, map_while.rs:49,
// scan.rs:51).
function $upperOnly(source) {
  return [0, $sizeHint(source)[1]];
}

// Std's `Take::size_hint` of what it has left to take, `n`, of `source`
// (take.rs:73): none, without asking `source`, where it has none left;
// else the fewer of its source's and `n`, both ways, `n` its upper bound
// where its source has none.
function $takeHint(source, n) {
  if (n === 0) return [0, 0];
  const [low, high] = $sizeHint(source);
  return [Math.min(low, n), high === undefined ? n : Math.min(high, n)];
}

// Two hints' sum, std's `Chain`'s and `Peekable`'s (chain.rs:182,
// peekable.rs:76): the lower bounds' a `usize`'s `saturating_add`, the
// upper ones' its `checked_add`, none past `usize::MAX` or where either
// has none.
function $hintSum([aLow, aHigh], [bLow, bHigh]) {
  const max = 2 ** 32 - 1;
  const high = aHigh === undefined || bHigh === undefined || aHigh + bHigh > max ? undefined : aHigh + bHigh;
  return [Math.min(aLow + bLow, max), high];
}

// Std's `FlattenCompat::size_hint` (flatten.rs:534): what's left of the
// inner iterators it's in, `front` and `back`, each asked where it has
// one, then its source's, `source`, fused, so `undefined` once it's done
// with it. Where its source may give more, each of which may give any
// number, no upper bound.
function $flatHint(front, source, back) {
  const none = [0, 0];
  const [low, high] = $hintSum(front === undefined ? none : $sizeHint(front), back === undefined ? none : $sizeHint(back));
  if (source === undefined) return [low, high];
  const [sourceLow, sourceHigh] = $sizeHint(source);
  return [low, sourceLow === 0 && sourceHigh === 0 ? high : undefined];
}
