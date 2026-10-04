// `a.eq_ignore_ascii_case(b)` of bytes: the same length, each byte equal once
// `A`..`Z` are lowered, as Rust's `to_ascii_lowercase` does.
function $bytesAsciiEq(a, b) {
  const lower = (byte) => (byte >= 65 && byte <= 90 ? byte + 32 : byte);
  return a.length === b.length && a.every((byte, i) => lower(byte) === lower(b[i]));
}
