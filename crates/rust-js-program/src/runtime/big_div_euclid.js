
function $bigDivEuclid(a, b, raisedAt, min) {
  const q = $bigDiv(a, b, raisedAt, min);
  if (a % b < 0n) return b > 0n ? q - 1n : q + 1n;
  return q;
}
