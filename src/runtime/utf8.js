// UTF-8 as Rust validates it (`core::str::validations`): the first bad
// sequence from `from`, `[at, length]`, its length `undefined` where the
// bytes end inside one, or `undefined` where there's none.
function $utf8Check(bytes, from = 0) {
  for (let i = from; i < bytes.length; ) {
    const first = bytes[i];
    if (first < 0x80) {
      i += 1;
      continue;
    }
    const width = first >= 0xc2 && first <= 0xdf ? 2 : first >= 0xe0 && first <= 0xef ? 3 : first >= 0xf0 && first <= 0xf4 ? 4 : 0;
    if (width === 0) return [i, 1];
    // The second byte's range rules out overlongs, surrogates and past U+10FFFF.
    const low = first === 0xe0 ? 0xa0 : first === 0xf0 ? 0x90 : 0x80;
    const high = first === 0xed ? 0x9f : first === 0xf4 ? 0x8f : 0xbf;
    for (let k = 1; k < width; k++) {
      if (i + k >= bytes.length) return [i, undefined];
      const byte = bytes[i + k];
      if (k === 1 ? byte < low || byte > high : (byte & 0xc0) !== 0x80) return [i, k];
    }
    i += width;
  }
  return undefined;
}

// Valid UTF-8's text, a U+FEFF at its start too, which `TextDecoder` would
// drop as a byte order mark, where Rust keeps every character.
function $utf8Decode(bytes) {
  return new TextDecoder("utf-8", { ignoreBOM: true }).decode(Uint8Array.from(bytes));
}

// `str::from_utf8(bytes)`, or `String::from_utf8(bytes)` (`owned`), whose
// `FromUtf8Error` keeps the bytes.
function $fromUtf8(bytes, owned = false) {
  const bad = $utf8Check(bytes);
  if (bad === undefined) return { TAG: "Ok", _0: $utf8Decode(bytes) };
  const error = { validUpTo: bad[0], errorLen: bad[1] };
  return { TAG: "Err", _0: owned ? { bytes, error } : error };
}

// `str::from_utf8_mut(bytes)` (ADR 0334): `Ok` of a `&mut str` read and
// written through `bytes`, which nothing it does makes longer or shorter,
// or `from_utf8`'s `Err`; `unchecked`, the `&mut str` itself.
function $fromUtf8Mut(bytes, unchecked = false) {
  const text = {
    get value() {
      return $utf8Decode(bytes);
    },
    set value(next) {
      new TextEncoder().encode(next).forEach((byte, i) => (bytes[i] = byte));
    },
  };
  if (unchecked) return text;
  const result = $fromUtf8(bytes);
  return result.TAG === "Ok" ? { TAG: "Ok", _0: text } : result;
}

// `String::from_utf8_lossy(bytes)`: a `Cow`, borrowed where the bytes are
// valid, else owned, with a U+FFFD for each bad sequence, which
// `TextDecoder` replaces as Rust's `Utf8Chunks` does, by its maximal subpart.
function $utf8Lossy(bytes) {
  return { TAG: $utf8Check(bytes) === undefined ? "Borrowed" : "Owned", _0: $utf8Decode(bytes) };
}

// A `Utf8Error`'s `Display`.
function $utf8ErrorMessage(error) {
  return error.errorLen === undefined
    ? `incomplete utf-8 byte sequence from index ${error.validUpTo}`
    : `invalid utf-8 sequence of ${error.errorLen} bytes from index ${error.validUpTo}`;
}

// A `Utf8Error`'s derived `Debug`, or a `FromUtf8Error`'s (`owned`).
function $debugUtf8Error(error, owned = false) {
  if (owned) return `FromUtf8Error { bytes: [${error.bytes.join(", ")}], error: ${$debugUtf8Error(error.error)} }`;
  const length = error.errorLen === undefined ? "None" : `Some(${error.errorLen})`;
  return `Utf8Error { valid_up_to: ${error.validUpTo}, error_len: ${length} }`;
}

// A byte slice's `escape_ascii()` (ADR 0341): each byte as `u8::escape_ascii`
// escapes it, `\t`, `\'` and `\x7f`, or itself where it's printable.
function $escapeAscii(bytes) {
  const named = { 9: "\\t", 10: "\\n", 13: "\\r", 34: '\\"', 39: "\\'", 92: "\\\\" };
  let text = "";
  for (const byte of bytes) {
    if (byte in named) text += named[byte];
    else if (byte >= 0x20 && byte < 0x7f) text += String.fromCharCode(byte);
    else text += `\\x${byte.toString(16).padStart(2, "0")}`;
  }
  return text;
}

// A byte's `escape_ascii()`, `ascii::escape_default(b)` (ADR 0558): the
// bytes of its escape, as `$escapeAscii` writes it.
function $escapeByte(byte) {
  return Array.from($escapeAscii([byte]), (c) => c.charCodeAt(0));
}

// `{}` of a byte's escape, stepped or not: the bytes it has left, as text.
function $restBytes(it) {
  return String.fromCharCode(...it.items.slice(it.at, it.end));
}

// `bytes.utf8_chunks()`: each run of valid text and the bad bytes after it,
// `{ valid, invalid }`, cut where Rust's `Utf8Chunks` cuts them: a sequence
// the bytes end inside of is one bad run.
function $utf8Chunks(bytes) {
  const chunks = [];
  for (let at = 0; at < bytes.length; ) {
    const bad = $utf8Check(bytes, at);
    const end = bad ? bad[0] : bytes.length;
    const stop = !bad ? end : bad[1] === undefined ? bytes.length : end + bad[1];
    chunks.push({ valid: $utf8Decode(bytes.slice(at, end)), invalid: bytes.slice(end, stop) });
    at = stop;
  }
  return chunks;
}
