
function $bigRemEuclid(a, b, raisedAt, min) {
  const r = $bigRem(a, b, raisedAt, min);
  return r < 0n ? (b < 0n ? r - b : r + b) : r;
}
