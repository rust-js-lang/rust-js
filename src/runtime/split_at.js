// `s.split_at(at)`: before and after the byte `at`, panicking as Rust's
// `&s[..at]` does.
function $splitAt(s, at) {
  return [$strSlice(s, 0, at), $strSlice(s, at)];
}
