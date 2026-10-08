
function $replace(s, pattern, replacement) {
  // A function's text is taken as it is, where a string's `$&` is the match.
  if (pattern !== "") return s.replaceAll(pattern, () => replacement);
  return Array.from(s, (c) => replacement + c).join("") + replacement;
}
function $split(s, pattern) {
  return pattern === "" ? ["", ...s, ""] : s.split(pattern);
}
