// `a.wrapping_rem(b)`, a number's or a BigInt's: `MIN % -1` is 0, where it
// would overflow; a zero `b` panics as `%` does.
// Raised where it was called, `raisedAt` (#2292).
function $wrappingRem(a, b, raisedAt, min) {
  if (b == 0) throw $panic("attempt to calculate the remainder with a divisor of zero", raisedAt);
  if (a === min && b == -1) return typeof a === "bigint" ? 0n : 0;
  const rem = a % b;
  return typeof rem === "bigint" ? rem : rem + 0;
}

// `a.overflowing_rem(b)`: what `wrapping_rem` gives, and whether it wrapped,
// `MIN % -1`'s.
function $overflowingRem(a, b, raisedAt, min) {
  return [$wrappingRem(a, b, raisedAt, min), a === min && b == -1];
}
