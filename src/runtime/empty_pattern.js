
function $replace(s, pattern, replacement) {
  // A function's text is taken as it is, where a string's `$&` is the match.
  if (pattern !== "") return s.replaceAll(pattern, () => replacement);
  return Array.from(s, (c) => replacement + c).join("") + replacement;
}
function $split(s, pattern) {
  return pattern === "" ? ["", ...s, ""] : s.split(pattern);
}
// `s.replacen(from, to, n)`: the first `n` matches, left to right; of an
// empty pattern, at each char's boundary, as Rust matches it.
function $replacen(s, from, to, n) {
  let out = "";
  let at = 0;
  if (from === "") {
    const chars = Array.from(s);
    for (let i = 0; i <= chars.length; i++) {
      if (i < n) out += to;
      if (i < chars.length) out += chars[i];
    }
    return out;
  }
  for (let count = 0; count < n; count++) {
    const found = s.indexOf(from, at);
    if (found < 0) break;
    out += s.slice(at, found) + to;
    at = found + from.length;
  }
  return out + s.slice(at);
}
