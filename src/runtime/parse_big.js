// `s.parse()` of an `i64` or a `u64`, or `from_str_radix(s, radix)`, as
// `$parseInt` reads a narrower integer, its value a BigInt.
function $parseBig(s, min, max, radix = 10) {
  if (radix < 2 || radix > 36) {
    throw new Error("from_ascii_radix: radix must lie in the range `[2, 36]` - found " + radix);
  }
  const error = (message) => ({ TAG: "Err", _0: message });
  if (s === "") return error("cannot parse integer from empty string");
  if (s === "+" || s === "-") return error("invalid digit found in string");
  const negative = s[0] === "-" && min < 0n;
  const digits = s[0] === "+" || negative ? s.slice(1) : s;
  const overflow = () =>
    error(negative ? "number too small to fit in target type" : "number too large to fit in target type");
  let n = 0n;
  for (const c of digits) {
    const digit = /^[0-9a-z]$/i.test(c) ? parseInt(c, 36) : radix;
    if (digit >= radix) return error("invalid digit found in string");
    n *= BigInt(radix);
    if (negative ? n < min : n > max) return overflow();
    n = negative ? n - BigInt(digit) : n + BigInt(digit);
    if (negative ? n < min : n > max) return overflow();
  }
  return { TAG: "Ok", _0: n };
}
