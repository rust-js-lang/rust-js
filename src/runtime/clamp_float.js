// `x.clamp(min, max)` of a float: `min` or `max` if it's past either, and a
// NaN itself. `min` past `max`, or either a NaN, panics, showing each with
// `debug`, as `{:?}` shows an `f64` or an `f32`.
function $clampFloat(x, min, max, debug) {
  if (!(min <= max)) {
    throw new Error("min > max, or either was NaN. min = " + debug(min) + ", max = " + debug(max));
  }
  return x < min ? min : x > max ? max : x;
}
