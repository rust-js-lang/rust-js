// `a.div_ceil(b)` of an unsigned integer, a number or a BigInt: `a / b`,
// rounded up.
function $divCeil(a, b) {
  if (b == 0) throw new Error("attempt to divide by zero");
  if (typeof a === "bigint") return a / b + (a % b > 0n ? 1n : 0n);
  return Math.trunc(a / b) + (a % b > 0 ? 1 : 0);
}
