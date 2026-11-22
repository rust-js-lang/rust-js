
// Raised where it was called, `raisedAt` (#2292).
function $bigRem(a, b, raisedAt, min) {
  if (b === 0n) {
    throw $panic("attempt to calculate the remainder with a divisor of zero", raisedAt);
  }
  if (a === min && b === -1n) {
    throw $panic("attempt to calculate the remainder with overflow", raisedAt);
  }
  return a % b;
}
