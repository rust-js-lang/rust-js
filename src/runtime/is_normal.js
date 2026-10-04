// `x.is_normal()`, or `is_subnormal()` (`subnormal`), of a float whose
// smallest normal value is `min`: an `f64`'s, or an `f32`'s, exactly.
function $isNormal(x, min, subnormal = false) {
  const size = Math.abs(x);
  return subnormal ? size > 0 && size < min : Number.isFinite(x) && size >= min;
}
