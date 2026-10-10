
// `make_ascii_uppercase()` of a `char` or a string, or of the bytes
// `start..end` of one, as `s[start..end]`'s, checked as that is (ADR 0334).
function $asciiCase(text, upper = false, start, end) {
  if (start !== undefined) {
    end ??= $byteLen(text);
    const part = $strSlice(text, start, end);
    return $strSlice(text, 0, start) + $asciiCase(part, upper) + $strSlice(text, end);
  }
  return upper
    ? text.replace(/[a-z]+/g, (letters) => letters.toUpperCase())
    : text.replace(/[A-Z]+/g, (letters) => letters.toLowerCase());
}
