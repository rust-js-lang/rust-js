
// `make_ascii_uppercase()` of a `char` or a string, or of the bytes
// `start..end` of one, as `s[start..end]`'s, checked as that is (ADR 0334).
function $asciiCase(text, upper = false) {
  return upper
    ? text.replace(/[a-z]+/g, (letters) => letters.toUpperCase())
    : text.replace(/[A-Z]+/g, (letters) => letters.toLowerCase());
}

// `s[start..end].make_ascii_uppercase()` (`upper`) or `_lowercase()` of a
// `String`: its text, that part's letters changed, checked as `&mut
// s[start..end]` is where it was made, `raisedAt` (#2292).
function $asciiCaseRange(text, upper, start, raisedAt, end = $byteLen(text), inclusive) {
  const part = $strSlice(text, start, raisedAt, end, inclusive);
  return $strSlice(text, 0, undefined, start) + $asciiCase(part, upper) + $strSlice(text, end);
}
