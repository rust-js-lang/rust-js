function $tryFromInt(x, lo, hi) {
  if (x < lo) return { TAG: "Err", _0: "number too small to fit in target type" };
  if (x > hi) return { TAG: "Err", _0: "number too large to fit in target type" };
  return { TAG: "Ok", _0: typeof hi === "bigint" ? BigInt(x) : Number(x) };
}

// `char::try_from(n)` of a `u32`: `Ok` of its `char`, or the error a
// surrogate's or one past `char::MAX`'s is, its message (ADR 0063).
function $charTryFrom(n) {
  const c = $fromU32(n);
  return c === undefined ? { TAG: "Err", _0: "converted integer out of range for `char`" } : { TAG: "Ok", _0: c };
}

// `u8::try_from(c)` of a `char`: `Ok` of its code point up to 255, or the
// error, its message.
function $u8TryFromChar(c) {
  const n = c.codePointAt(0);
  return n > 255 ? { TAG: "Err", _0: "unicode code point out of range" } : { TAG: "Ok", _0: n };
}
