// A float's `is_sign_negative()`: below zero, or -0. A NaN's sign isn't kept
// by JS, so a NaN is positive, as `f64::NAN` is.
function $signNegative(x) {
  return x < 0 || Object.is(x, -0);
}
