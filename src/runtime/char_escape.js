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

// `char::decode_utf16(units)`, std's `DecodeUtf16`: an iterator of each
// `char`, `Ok`, or `Err` of an unpaired surrogate, its code, each stepped
// of `units` as it's asked, one unit or a pair, a unit it read past an
// unpaired leading surrogate kept for its next step, as std's `buf` is; its
// hint std's; closed, `units` closed (#1151).
function $decodeUtf16(units) {
  // An array's `$iter`, which knows its hint, as a `Vec`'s or an array's
  // `IntoIter` does.
  const it = Array.isArray(units) ? $iter(units) : typeof units.next === "function" ? units : units[Symbol.iterator]();
  let buf;
  const decoded = new $Steps(
    () => {
      let unit = buf;
      buf = undefined;
      if (unit === undefined) {
        const step = it.next();
        if (step.done) return step;
        unit = step.value;
      }
      if (unit < 0xd800 || unit > 0xdfff) return { done: false, value: { TAG: "Ok", _0: String.fromCharCode(unit) } };
      if (unit >= 0xdc00) return { done: false, value: { TAG: "Err", _0: unit } };
      const next = it.next();
      if (next.done) return { done: false, value: { TAG: "Err", _0: unit } };
      if (next.value < 0xdc00 || next.value > 0xdfff) {
        buf = next.value;
        return { done: false, value: { TAG: "Err", _0: unit } };
      }
      return { done: false, value: { TAG: "Ok", _0: String.fromCharCode(unit, next.value) } };
    },
    () => it.return?.(),
  );
  decoded.from = [it];
  // Std's: half its units' lower bound, rounded up, and the one it kept.
  decoded.sizeHint = () => {
    const [low, high] = $sizeHint(it);
    const kept = buf === undefined ? 0 : 1;
    return [Math.ceil(low / 2) + kept, high === undefined ? undefined : high + kept];
  };
  return decoded;
}
