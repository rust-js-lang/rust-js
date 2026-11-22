// `x.clamp(min, max)` of a float: `min` or `max` if it's past either, and a
// NaN itself. `min` past `max`, or either a NaN, panics, showing each with
// `debug`, as `{:?}` shows an `f64` or an `f32`.
function $clampFloat(x, min, max, type) {
  if (!(min <= max)) {
    const debug = type === "f32" ? $debugF32 : $debugF64;
    throw $stringPanic("min > max, or either was NaN. min = " + debug(min) + ", max = " + debug(max), $stdSites.clamp[type][0]);
  }
  return x < min ? min : x > max ? max : x;
}
