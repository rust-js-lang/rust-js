
// Raised where it was called, `raisedAt` (#2292).
function $rem(a, b, raisedAt, min) {
  if (b === 0) {
    throw $panic("attempt to calculate the remainder with a divisor of zero", raisedAt);
  }
  if (a === min && b === -1) {
    throw $panic("attempt to calculate the remainder with overflow", raisedAt);
  }
  return a % b;
}
