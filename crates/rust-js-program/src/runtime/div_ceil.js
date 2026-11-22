// `a.div_ceil(b)` of an unsigned integer, a number or a BigInt: `a / b`,
// rounded up.
// Raised where it was called, `raisedAt` (#2292).
function $divCeil(a, b, raisedAt) {
  if (b == 0) throw $panic("attempt to divide by zero", raisedAt);
  if (typeof a === "bigint") return a / b + (a % b > 0n ? 1n : 0n);
  return Math.trunc(a / b) + (a % b > 0 ? 1 : 0);
}
