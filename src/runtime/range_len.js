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

// How many numbers `a..=b` has left, as `$rangeLen` of `a..b`.
function $rangeInclusiveLen(range) {
  return range.start <= range.end ? range.end - range.start + 1 : 0;
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
