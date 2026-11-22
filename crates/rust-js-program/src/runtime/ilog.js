// `x.ilog2()` or `x.ilog10()` of an integer, a number or a BigInt: how many
// times `base` divides into it, rounded down; `checked_ilog10()`'s
// `undefined`, `None`, where it isn't positive.
// `checked_ilog(base)`: `None` of what has none.
function $checkedIlog(x, base) {
  return x <= 0 || base < 2 ? undefined : $ilog(x, base, undefined);
}
// Raised where it was called, `raisedAt` (#2292).
function $ilog(x, base, raisedAt) {
  if (x <= 0) throw $panic("argument of integer logarithm must be positive", raisedAt);
  if (base < 2) throw $panic("base of integer logarithm must be at least 2", raisedAt);
  let log = 0;
  if (typeof x === "bigint") {
    for (const b = BigInt(base); x >= b; x /= b) log++;
  } else {
    for (; x >= base; x = Math.floor(x / base)) log++;
  }
  return log;
}
