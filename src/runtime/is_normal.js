// `x.is_normal()`, or `is_subnormal()` (`subnormal`), of a float whose
// smallest normal value is `min`: an `f64`'s, or an `f32`'s, exactly.
function $isNormal(x, min, subnormal = false) {
  const size = Math.abs(x);
  return subnormal ? size > 0 && size < min : Number.isFinite(x) && size >= min;
}

// `x.classify()`: its `FpCategory`, a variant's name, as an enum without
// fields is (ADR 0013).
function $classify(x, min) {
  if (Number.isNaN(x)) return "Nan";
  if (!Number.isFinite(x)) return "Infinite";
  if (x === 0) return "Zero";
  return Math.abs(x) < min ? "Subnormal" : "Normal";
}
