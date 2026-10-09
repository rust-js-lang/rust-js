
// One `char` as `{:?}` and `escape_debug()` show it: `quotes` the quotes it
// escapes, `extend` whether a Grapheme_Extend one is escaped too.
function $debugChar(c, quotes, extend = true) {
  if (quotes.includes(c) || c === "\\") return "\\" + c;
  if (c === "\n") return "\\n";
  if (c === "\r") return "\\r";
  if (c === "\t") return "\\t";
  if (c === "\0") return "\\0";
  // Halfwidth katakana's voiced marks, which 1.99 shows as they are,
  // though Unicode makes them combining marks.
  if (c === "\uff9e" || c === "\uff9f") return c;
  if (
    (extend && /\p{Grapheme_Extend}/u.test(c)) ||
    /[\p{Cc}\p{Cf}\p{Cs}\p{Co}\p{Cn}\p{Zl}\p{Zp}\p{Default_Ignorable_Code_Point}]/u.test(c) ||
    (c !== " " && /\p{Zs}/u.test(c))
  )
    return "\\u{" + c.codePointAt(0).toString(16) + "}";
  return c;
}

function $debugStr(s, quote = '"') {
  let out = quote;
  for (const c of s) out += $debugChar(c, quote);
  return out + quote;
}
