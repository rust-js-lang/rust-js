// How many numbers an exclusive range, `a..b`, has left: an
// `ExactSizeIterator`'s `len()`, of its state (ADR 0456), and `count()`'s,
// without stepping each, as std's is.
function $rangeLen(range) {
  return range.start < range.end ? range.end - range.start : 0;
}

// `r.nth(n)` of `a..b`: the number `n` on from its start, which its start
// moves past, or none, its start moved to its end, as std's `Range::nth`
// does, of numbers or BigInts, without stepping each.
function $rangeNth(range, n) {
  const big = typeof range.start === "bigint";
  const at = big ? range.start + BigInt(n) : range.start + n;
  if (at < range.end) {
    range.start = big ? at + 1n : at + 1;
    return at;
  }
  range.start = range.end;
  return undefined;
}

// `r.last()` of `a..b`: its last number, or none.
function $rangeLast(range) {
  if (!(range.start < range.end)) return undefined;
  return typeof range.end === "bigint" ? range.end - 1n : range.end - 1;
}

// `r.nth_back(n)` of `a..b`, which `r.rev().nth(n)` is: the number `n` back
// from its end, which its end moves to, or none, its end moved to its start.
function $rangeNthBack(range, n) {
  const at = typeof range.end === "bigint" ? range.end - 1n - BigInt(n) : range.end - 1 - n;
  if (at >= range.start) {
    range.end = at;
    return at;
  }
  range.end = range.start;
  return undefined;
}

// `r.nth(n)` of `a..=b`: as `$rangeNth`'s, its end the last number, and its
// start past its end once it's given all.
function $rangeInclusiveNth(range, n) {
  const big = typeof range.start === "bigint";
  const at = big ? range.start + BigInt(n) : range.start + n;
  if (at <= range.end) {
    range.start = big ? at + 1n : at + 1;
    return at;
  }
  range.start = big ? range.end + 1n : range.end + 1;
  return undefined;
}

// `r.nth_back(n)` of `a..=b`, which `r.rev().nth(n)` is.
function $rangeInclusiveNthBack(range, n) {
  const big = typeof range.end === "bigint";
  const at = big ? range.end - BigInt(n) : range.end - n;
  if (at >= range.start) {
    range.end = big ? at - 1n : at - 1;
    return at;
  }
  range.end = big ? range.start - 1n : range.start - 1;
  return undefined;
}

// `r.last()` of `a..=b`: its end, or none.
function $rangeInclusiveLast(range) {
  return range.start <= range.end ? range.end : undefined;
}

// `r.rev().last()` of `a..b`: its start, or none.
function $rangeFirst(range) {
  return range.start < range.end ? range.start : undefined;
}

// `r.rev().last()` of `a..=b`: its start, or none.
function $rangeInclusiveFirst(range) {
  return range.start <= range.end ? range.start : undefined;
}

// `count()` of `a..b` lent, `r.by_ref().count()`: how many numbers it has
// left, its start then its end, as std's `fold` of a `&mut` leaves it
// (#414).
function $rangeCountOut(range) {
  const left = $rangeLen(range);
  if (range.start < range.end) range.start = range.end;
  return left;
}

// `count()` of `a..=b` lent: how many, its start then past its end.
function $rangeInclusiveCountOut(range) {
  const left = $rangeInclusiveLen(range);
  if (range.start <= range.end) range.start = typeof range.end === "bigint" ? range.end + 1n : range.end + 1;
  return left;
}

// `last()` of `a..b` lent: its last number, or none, its start then its end.
function $rangeLastOut(range) {
  const last = $rangeLast(range);
  if (range.start < range.end) range.start = range.end;
  return last;
}

// `last()` of `a..=b` lent: its end, or none, its start then past its end.
function $rangeInclusiveLastOut(range) {
  const last = $rangeInclusiveLast(range);
  if (range.start <= range.end) range.start = typeof range.end === "bigint" ? range.end + 1n : range.end + 1;
  return last;
}

// How many numbers `a..=b` has left, as `$rangeLen` of `a..b`.
function $rangeInclusiveLen(range) {
  return range.start <= range.end ? range.end - range.start + 1 : 0;
}

// The `size_hint` of a range stepped by `next`, `$rangeNext`'s or its
// inclusive or back form's: std's, exact, of how many numbers it has left
// (ADR 0918); of a `char`'s or a BigInt's, none known, as before. Of `n..`,
// `$rangeFromNext`'s or `$rangeFromStep`'s, `usize::MAX` and none known,
// std's `RangeFrom`'s, whatever it's of (#400). `undefined` of another
// `next`.
function $rangeHintOf(next) {
  if (next === $rangeFromNext || next?.from === true) return () => [4294967295, undefined];
  const len =
    next === $rangeNext || next === $rangeNextBack
      ? $rangeLen
      : next === $rangeInclusiveNext || next === $rangeInclusiveNextBack
        ? $rangeInclusiveLen
        : undefined;
  if (len === undefined) return undefined;
  return (range) => {
    if (typeof range.start !== "number") return [0, undefined];
    const left = len(range);
    return [left, left];
  };
}

// The `nth` of a range stepped by `next`, `$rangeNext`'s or its inclusive
// or back form's: std's `Range::nth`, of its bounds, which `skip` and
// `step_by` step it by, so they don't step each number (#231); of a
// `char`'s, which steps by code point, each stepped. `undefined` of another
// `next`.
function $rangeNthOf(next) {
  const nth =
    next === $rangeNext
      ? $rangeNth
      : next === $rangeInclusiveNext
        ? $rangeInclusiveNth
        : next === $rangeNextBack
          ? $rangeNthBack
          : next === $rangeInclusiveNextBack
            ? $rangeInclusiveNthBack
            : undefined;
  if (nth === undefined) return undefined;
  return (range, k) => {
    if (typeof range.start !== "string") return nth(range, k);
    for (; k > 0; k--) if (next(range) === undefined) return undefined;
    return next(range);
  };
}
