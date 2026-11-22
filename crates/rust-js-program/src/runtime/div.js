
// Raised where it was called, `raisedAt` (#2292).
function $div(a, b, raisedAt, min) {
  if (b === 0) {
    throw $panic("attempt to divide by zero", raisedAt);
  }
  if (a === min && b === -1) {
    throw $panic("attempt to divide with overflow", raisedAt);
  }
  return a / b;
}
