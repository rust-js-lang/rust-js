// A float's `rem_euclid(rhs)`, or `div_euclid(rhs)`, as std computes it: the
// remainder made non-negative, and the quotient that goes with it, of
// `self / rhs` as its type rounds it, `round`, an `f32`'s `Math.fround`,
// before it's truncated (#940).
function $floatEuclid(a, b, rem, round) {
  const r = a % b;
  if (rem) return r < 0 ? r + Math.abs(b) : r;
  const q = Math.trunc(round === undefined ? a / b : round(a / b));
  if (r < 0) return b > 0 ? q - 1 : q + 1;
  return q;
}

// A float's `abs_sub(other)`, C's `fdim`: the difference where it's
// positive, else 0, and a NaN of either a NaN.
function $absSub(a, b) {
  if (Number.isNaN(a) || Number.isNaN(b)) return NaN;
  return a > b ? a - b : 0;
}
