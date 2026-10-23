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
