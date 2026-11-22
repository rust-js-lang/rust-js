// `x.saturating_pow(exp)`: the power, or the bound it overflows past, `lo`
// for a negative base to an odd power.
function $saturatingPow(base, exp, lo, hi) {
  const power = $checkedPow(base, exp, lo, hi);
  if (power !== undefined) return power;
  return base < 0 && exp % 2 === 1 ? lo : hi;
}
