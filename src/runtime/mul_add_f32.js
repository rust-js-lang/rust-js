// `a.mul_add(b, c)` of `f32`s: `a * b + c`, rounded once to an `f32`
// (ADR 0430).
function $mulAddF32(a, b, c) {
  return Math.fround($fused(a, b, c, 24, -149));
}
