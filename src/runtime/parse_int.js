// `s.parse()` of an integer, or `from_str_radix(s, radix)`, as std reads it:
// a sign, then each digit in turn, stopping at the first that isn't one or
// overflows, so `"999x"` overflows a `u8` before its `x` is read. Each digit
// is read before what's so far, times the radix, is checked: `"26x"` is
// an invalid digit, though 260 is too large.
function $parseInt(s, min, max, radix = 10) {
  if (radix < 2 || radix > 36) {
    throw new Error("from_ascii_bytes_radix: radix must lie in the range `[2, 36]` - found " + radix);
  }
  const error = (message) => ({ TAG: "Err", _0: message });
  if (s === "") return error("cannot parse integer from empty string");
  if (s === "+" || s === "-") return error("invalid digit found in string");
  const negative = s[0] === "-" && min < 0;
  const digits = s[0] === "+" || negative ? s.slice(1) : s;
  const overflow = () =>
    error(negative ? "number too small to fit in target type" : "number too large to fit in target type");
  let n = 0;
  for (const c of digits) {
    const digit = /^[0-9a-z]$/i.test(c) ? parseInt(c, 36) : radix;
    if (digit >= radix) return error("invalid digit found in string");
    n *= radix;
    if (negative ? n < min : n > max) return overflow();
    n = negative ? n - digit : n + digit;
    if (negative ? n < min : n > max) return overflow();
  }
  return { TAG: "Ok", _0: n };
}
