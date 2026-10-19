// `x.ilog2()` or `x.ilog10()` of an integer, a number or a BigInt: how many
// times `base` divides into it, rounded down; `checked_ilog10()`'s
// `undefined`, `None`, where it isn't positive.
function $ilog(x, base, checked = false) {
  if (x <= 0) {
    if (checked) return undefined;
    throw new Error("argument of integer logarithm must be positive");
  }
  let log = 0;
  if (typeof x === "bigint") {
    for (const b = BigInt(base); x >= b; x /= b) log++;
  } else {
    for (; x >= base; x = Math.floor(x / base)) log++;
  }
  return log;
}
