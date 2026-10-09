// A `char`'s or a `str`'s escapes, as text, and a `char`'s UTF-8 and UTF-16
// (ADR 0327).

// `escape_debug()`: each `char` as `{:?}` shows it, both quotes escaped; a
// `str`'s Grapheme_Extend ones only first.
function $escapeDebug(s, str) {
  let out = "";
  let first = true;
  for (const c of s) {
    out += $debugChar(c, "'\"", !str || first);
    first = false;
  }
  return out;
}

// `escape_default()`: `\t`, `\r`, `\n`, quotes and `\\` escaped, printable
// ASCII as it is, and the rest as `\u{..}`.
function $escapeDefault(s) {
  let out = "";
  for (const c of s) {
    if (c === "\t") out += "\\t";
    else if (c === "\r") out += "\\r";
    else if (c === "\n") out += "\\n";
    else if (c === "'" || c === '"' || c === "\\") out += "\\" + c;
    else if (c >= " " && c <= "~") out += c;
    else out += "\\u{" + c.codePointAt(0).toString(16) + "}";
  }
  return out;
}

function $escapeUnicode(s) {
  let out = "";
  for (const c of s) out += "\\u{" + c.codePointAt(0).toString(16) + "}";
  return out;
}

// `c.encode_utf8(&mut buf)`: its bytes at the start of `buf`, and itself;
// a `buf` too short panics, as std's does.
function $encodeUtf8(c, buf) {
  const bytes = new TextEncoder().encode(c);
  if (bytes.length > buf.length) {
    const code = c.codePointAt(0).toString(16).toUpperCase().padStart(4, "0");
    throw new Error(`encode_utf8: need ${bytes.length} bytes to encode U+${code} but buffer has just ${buf.length}`);
  }
  bytes.forEach((b, i) => (buf[i] = b));
  return c;
}

// `char::decode_utf16(units)`: each `char`, `Ok`, or `Err` of an unpaired
// surrogate, its code.
function $decodeUtf16(units) {
  const out = [];
  const list = Array.from(units);
  for (let i = 0; i < list.length; i++) {
    const unit = list[i];
    if (unit < 0xd800 || unit > 0xdfff) out.push({ TAG: "Ok", _0: String.fromCharCode(unit) });
    else if (unit <= 0xdbff && i + 1 < list.length && list[i + 1] >= 0xdc00 && list[i + 1] <= 0xdfff) {
      out.push({ TAG: "Ok", _0: String.fromCharCode(unit, list[i + 1]) });
      i++;
    } else out.push({ TAG: "Err", _0: unit });
  }
  return out;
}
