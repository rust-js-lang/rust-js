// `x.clamp(min, max)` of an integer, a number or a BigInt: `min` or `max` if
// it's past either. Bounds the wrong way round panic as std's integers do,
// each shown as `{:?}` shows it, which is its digits.
function $clamp(x, min, max) {
  if (!(min <= max)) throw new Error("min > max. min = " + min + ", max = " + max);
  return x < min ? min : x > max ? max : x;
}
