// An `a..`'s `next()`, `{ start }`: its start, moved past. Of an integer,
// as release Rust's, wrapped past `hi`, its type's last, to `lo`, its
// first (ADR 0011, #414); of a `char`, by its code point, past the
// surrogates, and past the last `char` std's panic, before it gives one.
function $rangeFromNext(range, lo, hi) {
  const n = range.start;
  if (typeof n === "string") {
    const next = $charStep(n, 1);
    if (next === undefined) throw new Error("overflow in `Step::forward`");
    range.start = next;
  } else {
    range.start = n === hi ? lo : typeof n === "bigint" ? n + 1n : n + 1;
  }
  return n;
}

// The `next` of an `a..` of integers from `lo` to `hi`, as a value, its
// `Iterator`'s: `$rangeFromNext` of those bounds, which `$rangeHintOf`
// knows as an `a..`'s by its `from`.
function $rangeFromStep(lo, hi) {
  const step = (range) => $rangeFromNext(range, lo, hi);
  step.from = true;
  return step;
}
