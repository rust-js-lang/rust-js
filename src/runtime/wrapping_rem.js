// `a.wrapping_rem(b)`, a number's or a BigInt's: `MIN % -1` is 0, where it
// would overflow; a zero `b` panics as `%` does.
function $wrappingRem(a, b, min) {
  if (b == 0) throw new Error("attempt to calculate the remainder with a divisor of zero");
  if (a === min && b == -1) return typeof a === "bigint" ? 0n : 0;
  const rem = a % b;
  return typeof rem === "bigint" ? rem : rem + 0;
}
