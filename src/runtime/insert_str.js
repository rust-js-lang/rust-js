// `s.insert(at, c)` and `s.insert_str(at, t)`: `t` at the byte `at`, which
// must start a `char` or be the end, as Rust asserts.
function $insertStr(s, at, t) {
  const unit = $charBoundary(s, at);
  if (unit === undefined) throw new Error("assertion failed: self.is_char_boundary(idx)");
  return s.slice(0, unit) + t + s.slice(unit);
}
