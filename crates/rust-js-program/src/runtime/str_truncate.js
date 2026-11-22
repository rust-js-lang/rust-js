// `s.truncate(n)`: its first `n` bytes, or all of it, if it has fewer;
// inside a `char`, Rust's assertion fails.
function $strTruncate(s, n) {
  if (n >= $byteLen(s)) return s;
  const unit = $charBoundary(s, n);
  if (unit === undefined) throw new Error("assertion failed: self.is_char_boundary(new_len)");
  return s.slice(0, unit);
}
