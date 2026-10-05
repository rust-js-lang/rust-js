// `s.trim()`, `trim_start()` and `trim_end()`, by Unicode's White_Space, as
// Rust's are: U+0085 trimmed, and U+FEFF kept, where JS's `trim()` does
// the other (ADR 0183).
function $trim(s) {
  return s.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, "");
}

function $trimStart(s) {
  return s.replace(/^\p{White_Space}+/u, "");
}

function $trimEnd(s) {
  return s.replace(/\p{White_Space}+$/u, "");
}
