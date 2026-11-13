// A C string (#1157, the user's decision): a `CStr` is its bytes with its
// nul, as a `[u8]` is, a JS array of numbers, `c"hi"` `[104, 105, 0]`; a
// `CString` its `Vec<u8>` with the nul. std's errors are as any of their
// kind is: an enum's variant its name or `{ TAG, .. }`, a tuple struct an
// array.

// `CStr::from_bytes_with_nul(bytes)`: `Ok` of the bytes, which end in their
// one nul; or `Err` of std's `FromBytesWithNulError`, `InteriorNul` at the
// first nul before the end, or `NotNulTerminated`.
function $cstrWithNul(bytes) {
  const nul = bytes.indexOf(0);
  if (nul === bytes.length - 1) return { TAG: "Ok", _0: bytes };
  return { TAG: "Err", _0: nul < 0 ? "NotNulTerminated" : { TAG: "InteriorNul", position: nul } };
}

// `CStr::from_bytes_until_nul(bytes)`: `Ok` of the bytes to the first nul,
// with it, a part as `&bytes[..=nul]` is; or `Err` of a
// `FromBytesUntilNulError`, of none.
function $cstrUntilNul(bytes) {
  const nul = bytes.indexOf(0);
  if (nul < 0) return { TAG: "Err", _0: [undefined] };
  return { TAG: "Ok", _0: nul === bytes.length - 1 ? bytes : $part(bytes, 0, nul + 1) };
}

// `c.to_bytes()`: its bytes without the nul, a part as `&c[..n]` is.
function $cstrBytes(c) {
  return $part(c, 0, c.length - 1);
}

// `CString::new(t)`, of `t`'s bytes: `Ok` of them and a nul, or `Err` of a
// `NulError`, the first nul's position and the bytes. Each made anew, as
// what was given may be a `Vec` another holds too, where nothing changes
// one in place (ADR 0052).
function $cstringNew(bytes) {
  const nul = bytes.indexOf(0);
  if (nul >= 0) return { TAG: "Err", _0: [nul, bytes] };
  return { TAG: "Ok", _0: bytes.concat([0]) };
}

// `CString::from_vec_unchecked(v)`: `v`'s bytes and a nul.
function $cstringFromVec(bytes) {
  return bytes.concat([0]);
}

// `CString::from_vec_with_nul(v)`: `Ok` of `v`, which ends in its one nul;
// or `Err` of a `FromVecWithNulError`, its kind as `from_bytes_with_nul`'s
// and the bytes.
function $cstringWithNul(bytes) {
  const nul = bytes.indexOf(0);
  if (nul === bytes.length - 1) return { TAG: "Ok", _0: bytes };
  return { TAG: "Err", _0: { errorKind: nul < 0 ? "NotNulTerminated" : { TAG: "InteriorNul", _0: nul }, bytes } };
}

// `s.into_bytes()`, `Vec::from(s)`: its bytes without the nul.
function $cstringIntoBytes(c) {
  return c.slice(0, -1);
}

// `s.into_string()`: `Ok` of its text, or `Err` of an `IntoStringError`, the
// `CString` and its `Utf8Error`.
function $cstringIntoString(c) {
  const text = $fromUtf8($cstrBytes(c));
  return text.TAG === "Ok" ? text : { TAG: "Err", _0: { inner: c, error: text._0 } };
}

// `{:?}` of a C string, std's `ByteStr`'s of its bytes without the nul:
// valid text with each ASCII character escaped as `escape_ascii` does it,
// `\0` the nul's, another as `escape_debug` does it, and each invalid byte
// `\xNN`.
function $debugCStr(c) {
  const ascii = (bytes) => String.fromCharCode.apply(null, $escapeAscii(bytes));
  let out = '"';
  for (const { valid, invalid } of $utf8Chunks($cstrBytes(c))) {
    for (const ch of valid) {
      const code = ch.codePointAt(0);
      out += code === 0 ? "\\0" : code < 0x80 ? ascii([code]) : $escapeDebug(ch);
    }
    out += ascii(invalid);
  }
  return `${out}"`;
}

// A C string's error's `Display`, as std writes it: `kind` says which,
// `withNul`'s `FromBytesWithNulError`, `untilNul`'s
// `FromBytesUntilNulError`, a `NulError` or an `IntoStringError`.
function $cstrErrorMessage(error, kind) {
  switch (kind) {
    case "withNul":
      return error === "NotNulTerminated"
        ? "data provided is not nul terminated"
        : `data provided contains an interior nul byte at byte position ${error.position}`;
    case "untilNul":
      return "data provided does not contain a nul";
    case "nul":
      return `nul byte found in provided data at position: ${error[0]}`;
    case "fromVec":
      return error.errorKind === "NotNulTerminated"
        ? "data provided is not nul terminated"
        : `data provided contains an interior nul byte at pos ${error.errorKind._0}`;
    default:
      return "C string contained non-utf8 bytes";
  }
}

// A C string's error's derived `Debug`, its numbers padded as `options`
// say; with `{:#?}`, `options.alternate`, on lines of their own, indented.
function $debugCStrError(error, kind, options) {
  const n = (number) => $padIntegral(true, "", String(number), options);
  const pretty = options?.alternate;
  switch (kind) {
    case "withNul":
      if (error === "NotNulTerminated") return error;
      return pretty ? `InteriorNul {\n    position: ${n(error.position)},\n}` : `InteriorNul { position: ${n(error.position)} }`;
    case "untilNul":
      return pretty ? "FromBytesUntilNulError(\n    (),\n)" : "FromBytesUntilNulError(())";
    case "nul": {
      const [at, bytes] = error;
      if (!pretty) return `NulError(${n(at)}, [${bytes.map(n).join(", ")}])`;
      const items = bytes.length === 0 ? "[]" : `[\n${bytes.map((b) => `        ${n(b)},\n`).join("")}    ]`;
      return `NulError(\n    ${n(at)},\n    ${items},\n)`;
    }
    case "fromVec": {
      const { errorKind, bytes } = error;
      const kind = (open, close) =>
        errorKind === "NotNulTerminated" ? errorKind : `InteriorNul(${open}${n(errorKind._0)}${close})`;
      if (!pretty) return `FromVecWithNulError { error_kind: ${kind("", "")}, bytes: [${bytes.map(n).join(", ")}] }`;
      const items = bytes.length === 0 ? "[]" : `[\n${bytes.map((b) => `        ${n(b)},\n`).join("")}    ]`;
      return `FromVecWithNulError {\n    error_kind: ${kind("\n        ", ",\n    ")},\n    bytes: ${items},\n}`;
    }
    default: {
      const utf8 = $debugUtf8Error(error.error, false, options);
      if (!pretty) return `IntoStringError { inner: ${$debugCStr(error.inner)}, error: ${utf8} }`;
      return `IntoStringError {\n    inner: ${$debugCStr(error.inner)},\n    error: ${utf8.replaceAll("\n", "\n    ")},\n}`;
    }
  }
}

// `CStr::from_ptr(p)`: the bytes from where `p` points up to its nul, and
// the nul, a C string's (ADR 1119), each read as a `u8`: a copy, as what a
// shared C string borrows doesn't change while it's borrowed (#1291).
function $cstrFromPtr(pointer) {
  const bytes = [];
  for (let i = 0; ; i++) {
    const byte = $ptrAdd(pointer, i).value & 0xff;
    bytes.push(byte);
    if (byte === 0) return bytes;
  }
}

// `CString::from_raw(p)`: the C string whose first byte `into_raw` gave `p`
// a pointer to; of another pointer, which Rust leaves undefined, rust-js's
// error (#1291).
function $cstringFromRaw(pointer) {
  const at = $byteOffset(pointer);
  if (at !== undefined && at.at === 0) return at.of;
  throw $unsupported("rust-js does not support `CString::from_raw` of a pointer `CString::into_raw` didn't give, which reads memory by where it is");
}
