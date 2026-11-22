
function $divEuclid(a, b, raisedAt, min) {
  const q = Math.trunc($div(a, b, raisedAt, min)) + 0;
  return a % b < 0 ? (b > 0 ? q - 1 : q + 1) : q;
}
