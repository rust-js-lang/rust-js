// `x.ilog2()` or `x.ilog10()` of an integer, a number or a BigInt: how many
// times `base` divides into it, rounded down.
function $ilog(x, base) {
  if (x <= 0) throw new Error("argument of integer logarithm must be positive");
  let log = 0;
  if (typeof x === "bigint") {
    for (const b = BigInt(base); x >= b; x /= b) log++;
  } else {
    for (; x >= base; x = Math.floor(x / base)) log++;
  }
  return log;
}
