// A `RangeInclusive`'s `next()`, `{ start, end }`: its start, moved past,
// while it's at most its end. Past its end, as Rust's, it gives nothing; a
// `char`'s by its code point. Its number's past its type's last is left
// unwrapped, std's exhausted range, which its readers wrap (#414); past the
// last `char`, it's exhausted at it, as std's is.
function $rangeInclusiveNext(range) {
  if (typeof range.start === "string") {
    if (range.start.codePointAt(0) > range.end.codePointAt(0)) return undefined;
    const c = range.start;
    const next = $charStep(c, 1);
    if (next === undefined) $rangeInclusiveExhausted(range, c, range.end);
    else range.start = next;
    return c;
  }
  return range.start <= range.end ? range.start++ : undefined;
}

// An inclusive range used up as std's leaves one, its bounds Rust's,
// `start` and `end`, and exhausted: `exhausted` holds those, which its
// readers read, and its own bounds hold nothing, its start past its end, so
// what asks whether it's empty asks them as it is (#414).
function $rangeInclusiveExhausted(range, start, end) {
  range.exhausted = { start, end };
  if (typeof range.end !== "string") {
    range.start = typeof range.end === "bigint" ? range.end + 1n : range.end + 1;
  } else {
    const after = $charStep(range.end, 1);
    if (after !== undefined) range.start = after;
    else range.end = $charStep(range.start, -1) ?? range.end;
  }
}

// Std's `RangeInclusive::try_fold`: `f` of each number from its start,
// given what it gave, until `broke` says what it gave stops it; at its last,
// it's exhausted there, its start its end, before `f` of it, as std's
// leaves it, not past it as `next` does (#414). Of `char`s, by code point.
function $rangeInclusiveTryFold(range, init, f, broke) {
  const text = typeof range.start === "string";
  const at = (c) => (text ? c.codePointAt(0) : c);
  if (!(at(range.start) <= at(range.end))) return init;
  let folded = init;
  while (at(range.start) < at(range.end)) {
    const n = range.start;
    range.start = text ? $charStep(n, 1) : typeof n === "bigint" ? n + 1n : n + 1;
    folded = f(folded, n);
    if (broke(folded)) return folded;
  }
  const last = range.end;
  $rangeInclusiveExhausted(range, last, last);
  return f(folded, last);
}

// Std's `RangeInclusive::try_rfold`, from its end: at its first, it's
// exhausted there, its end its start.
function $rangeInclusiveTryRfold(range, init, f, broke) {
  const text = typeof range.start === "string";
  const at = (c) => (text ? c.codePointAt(0) : c);
  if (!(at(range.start) <= at(range.end))) return init;
  let folded = init;
  while (at(range.start) < at(range.end)) {
    const n = range.end;
    range.end = text ? $charStep(n, -1) : typeof n === "bigint" ? n - 1n : n - 1;
    folded = f(folded, n);
    if (broke(folded)) return folded;
  }
  const first = range.start;
  $rangeInclusiveExhausted(range, first, first);
  return f(folded, first);
}
