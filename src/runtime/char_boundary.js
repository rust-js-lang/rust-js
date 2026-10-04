// Where the byte `at` of `s` is, in UTF-16 units: if a `char` starts there,
// or it's the end. Otherwise `undefined`, as `is_char_boundary` is `false`.
function $charBoundary(s, at) {
  let bytes = 0;
  let unit = 0;
  for (const c of s) {
    if (bytes === at) return unit;
    if (bytes > at) return undefined;
    bytes += $byteLen(c);
    unit += c.length;
  }
  return bytes === at ? unit : undefined;
}
