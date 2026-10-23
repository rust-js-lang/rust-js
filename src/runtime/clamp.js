// `x.clamp(min, max)` of an integer, a number or a BigInt: `min` or `max` if
// it's past either. Bounds the wrong way round panic as std's integers do,
// each shown as `{:?}` shows it, which is its digits.
function $clamp(x, min, max) {
  if (!(min <= max)) throw new Error("min > max. min = " + min + ", max = " + max);
  return x < min ? min : x > max ? max : x;
}

// `x.clamp(min, max)` of what `cmp` orders, `Ord`'s: `min` or `max` if it's
// past either. Bounds the wrong way round panic as `Ord`'s default does,
// or, given `show`, as a `char`'s does, each as `{:?}` shows it.
function $clampBy(x, min, max, cmp, show) {
  if (cmp(min, max) > 0) {
    throw new Error(show ? `min > max. min = ${show(min)}, max = ${show(max)}` : "assertion failed: min <= max");
  }
  return cmp(x, min) < 0 ? min : cmp(x, max) > 0 ? max : x;
}
