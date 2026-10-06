
function $debugStr(s, quote = '"') {
  let out = quote;
  for (const c of s) {
    if (c === quote || c === "\\") out += "\\" + c;
    else if (c === "\n") out += "\\n";
    else if (c === "\r") out += "\\r";
    else if (c === "\t") out += "\\t";
    else if (c === "\0") out += "\\0";
    // Halfwidth katakana's voiced marks, which 1.99 shows as they are,
    // though Unicode makes them combining marks.
    else if (c === "\uff9e" || c === "\uff9f") out += c;
    else if (/[\p{Cc}\p{Cf}\p{Cs}\p{Co}\p{Cn}\p{Zl}\p{Zp}\p{Grapheme_Extend}\p{Default_Ignorable_Code_Point}]/u.test(c) || (c !== " " && /\p{Zs}/u.test(c)))
      out += "\\u{" + c.codePointAt(0).toString(16) + "}";
    else out += c;
  }
  return out + quote;
}
