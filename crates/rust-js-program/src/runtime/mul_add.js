// `a.mul_add(b, c)` of `f64`s: `a * b + c`, rounded once (ADR 0430).
function $mulAdd(a, b, c) {
  return $fused(a, b, c, 53, -1074);
}
