
// Raised where it was called, `raisedAt` (#2292).
function $bigDiv(a, b, raisedAt, min) {
  if (b === 0n) {
    throw $panic("attempt to divide by zero", raisedAt);
  }
  if (a === min && b === -1n) {
    throw $panic("attempt to divide with overflow", raisedAt);
  }
  return a / b;
}
