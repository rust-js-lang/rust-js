// An integer's `overflowing_*`: what wrapping gives, and whether the exact
// result was out of range.
function $overflowing(wrapped, exact, lo, hi) {
  return [wrapped, exact < lo || exact > hi];
}

// `a.pow(n)` exact, a number's or a BigInt's, the `exact` `$overflowing`
// compares: past 2^53 a number's is rounded, but it's out of range anyway.
function $powExact(a, n) {
  return typeof a === "bigint" ? a ** BigInt(n) : a ** n;
}

// An exact result kept in `lo` to `hi`, a `saturating_*`'s.
function $saturate(exact, lo, hi) {
  return exact < lo ? lo : exact > hi ? hi : exact;
}

// `a.eq_ignore_ascii_case(&b)` of two bytes: equal once each ASCII
// uppercase letter is its lowercase one.
function $bytesEqIgnoreAsciiCase(a, b) {
  const lower = (c) => (c >= 65 && c <= 90 ? c + 32 : c);
  return lower(a) === lower(b);
}
