// An integer's `overflowing_*`: what wrapping gives, and whether the exact
// result was out of range.
function $overflowing(wrapped, exact, lo, hi) {
  return [wrapped, exact < lo || exact > hi];
}
