// `s.trim()`, `trim_start()` and `trim_end()`, by `char::is_whitespace`'s
// table, Rust's (ADR 0183, #1888): U+0085 trimmed, and U+FEFF kept, where
// JS's `trim()` does the other.
function $trim(s) {
  return $trimEnd($trimStart(s));
}

function $trimStart(s) {
  return s.replace($unicodeRegexp($unicodeWhitespace, "^#+"), "");
}

function $trimEnd(s) {
  return s.replace($unicodeRegexp($unicodeWhitespace, "#+$"), "");
}
