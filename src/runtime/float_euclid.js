// A float's `rem_euclid(rhs)`, or `div_euclid(rhs)`, as std computes it: the
// remainder made non-negative, and the quotient that goes with it.
function $floatEuclid(a, b, rem) {
  const r = a % b;
  if (rem) return r < 0 ? r + Math.abs(b) : r;
  const q = Math.trunc(a / b);
  if (r < 0) return b > 0 ? q - 1 : q + 1;
  return q;
}

// A float's `abs_sub(other)`, C's `fdim`: the difference where it's
// positive, else 0, and a NaN of either a NaN.
function $absSub(a, b) {
  if (Number.isNaN(a) || Number.isNaN(b)) return NaN;
  return a > b ? a - b : 0;
}
