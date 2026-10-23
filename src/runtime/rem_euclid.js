
function $remEuclid(a, b, min) {
  const r = $rem(a, b, min) + 0;
  return r < 0 ? (b < 0 ? r - b : r + b) : r;
}

// `a.wrapping_rem_euclid(b)` and `wrapping_div_euclid`, a number's or a
// BigInt's: `MIN` by `-1`, which would overflow, is 0 and `MIN`; else the
// Euclidean remainder or quotient, a zero `b` panicking as `%` does.
function $wrappingRemEuclid(a, b, min) {
  if (a === min && b == -1) return typeof a === "bigint" ? 0n : 0;
  return typeof a === "bigint" ? $bigRemEuclid(a, b, min) : $remEuclid(a, b, min);
}

function $wrappingDivEuclid(a, b, min) {
  if (a === min && b == -1) return a;
  return typeof a === "bigint" ? $bigDivEuclid(a, b, min) : $divEuclid(a, b, min);
}
