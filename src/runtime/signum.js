// A float's `signum()`: 1 or -1 by its sign, -0's -1 too, or a NaN.
function $signum(x) {
  if (Number.isNaN(x)) return NaN;
  return x < 0 || Object.is(x, -0) ? -1 : 1;
}
