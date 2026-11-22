
// One `char` as `{:?}` and `escape_debug()` show it, `quotes` the quotes it
// escapes; one Rust's table says it escapes, `\u{..}` (#1888).
function $debugChar(c, quotes) {
  if (quotes.includes(c) || c === "\\") return "\\" + c;
  if (c === "\n") return "\\n";
  if (c === "\r") return "\\r";
  if (c === "\t") return "\\t";
  if (c === "\0") return "\\0";
  if ($unicodeRegexp($unicodeDebugEscaped).test(c)) return "\\u{" + c.codePointAt(0).toString(16) + "}";
  return c;
}

function $debugStr(s, quote = '"') {
  let out = quote;
  for (const c of s) out += $debugChar(c, quote);
  return out + quote;
}
