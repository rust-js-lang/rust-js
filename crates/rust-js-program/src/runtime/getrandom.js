// getrandom's, of a version the catalog qualified (#2326): its fills and
// numbers, Web Crypto's `crypto.getRandomValues`, and its `Error`, its code,
// a number, shown as that version's `Debug` and `Display` show it.

// Its `UNSUPPORTED`'s code, what a fill gives where there's no Web Crypto,
// of each version's `INTERNAL_START`: 0.1's and 0.2's `1 << 31`, 0.3's and
// 0.4's `1 << 16`.
const $getrandomInternalStart = { "0.1": 2 ** 31, "0.2": 2 ** 31, "0.3": 2 ** 16 };

// 0.1's and 0.2's `getrandom(dest)`, 0.3's and 0.4's `fill(dest)`, or of
// `uninit`, their `fill_uninit(dest)`: `dest`, an array of bytes, filled in
// place, 65,536 bytes at a time, as many as `getRandomValues` fills at
// once; `Ok` of nothing, or of `uninit`, of `dest`. Where there's no Web
// Crypto, `Err` of `UNSUPPORTED`, as the version's backend that has none
// gives, and `dest` as it was.
function $getrandom(dest, version, uninit = false) {
  const crypto = globalThis.crypto;
  if (typeof crypto?.getRandomValues !== "function") {
    return { TAG: "Err", _0: $getrandomInternalStart[version] };
  }
  const chunk = new Uint8Array(Math.min(dest.length, 65536));
  for (let at = 0; at < dest.length; at += chunk.length) {
    const part = dest.length - at < chunk.length ? chunk.subarray(0, dest.length - at) : chunk;
    crypto.getRandomValues(part);
    for (let i = 0; i < part.length; i++) dest[at + i] = part[i];
  }
  return uninit ? { TAG: "Ok", _0: dest } : { TAG: "Ok", _0: undefined };
}
// 0.3's and 0.4's `u32()` and `u64()`: a random number, a `u64` a BigInt;
// `Err` of `UNSUPPORTED` where there's no Web Crypto.
function $getrandomU32() {
  const crypto = globalThis.crypto;
  if (typeof crypto?.getRandomValues !== "function") return { TAG: "Err", _0: $getrandomInternalStart["0.3"] };
  return { TAG: "Ok", _0: crypto.getRandomValues(new Uint32Array(1))[0] };
}
function $getrandomU64() {
  const crypto = globalThis.crypto;
  if (typeof crypto?.getRandomValues !== "function") return { TAG: "Err", _0: $getrandomInternalStart["0.3"] };
  return { TAG: "Ok", _0: crypto.getRandomValues(new BigUint64Array(1))[0] };
}

// An `Error`'s `raw_os_error()`, of its code: 0.1's and 0.2's below
// `INTERNAL_START`, the code; 0.3's and 0.4's negative, its negation.
function $getrandomRawOsError(code, version) {
  if (version === "0.3") return code < 0 ? -code : undefined;
  return code < $getrandomInternalStart[version] ? code : undefined;
}

// What each version's `internal_desc` says of each internal code, by its
// number past `INTERNAL_START`: 0.1's and 0.2's tables, and 0.3's and
// 0.4's, whose other entries rustc's check of them for rust-js's target,
// by the custom backend, leaves out.
const $getrandomWords = {
  "0.1": [
    "getrandom: this target is not supported",
    "errno: did not return a positive value",
    "Unknown std::io::Error",
    "SecRandomCopyBytes: call failed",
    "RtlGenRandom: call failed",
    "RDRAND: failed multiple times: CPU issue likely",
    "RDRAND: instruction not supported",
    "wasm-bindgen: self.crypto is undefined",
    "wasm-bindgen: crypto.getRandomValues is undefined",
    "stdweb: no randomness source available",
    "stdweb: failed to get randomness",
    "randSecure: random number generator module is not initialized",
  ],
  "0.2": [
    "getrandom: this target is not supported",
    "errno: did not return a positive value",
    "unexpected situation",
    "SecRandomCopyBytes: iOS Security framework failure",
    "RtlGenRandom: Windows system function failure",
    "RDRAND: failed multiple times: CPU issue likely",
    "RDRAND: instruction not supported",
    "Web Crypto API is unavailable",
    "Calling Web API crypto.getRandomValues failed",
    undefined,
    undefined,
    "randSecure: VxWorks RNG module is not initialized",
    "Node.js crypto CommonJS module is unavailable",
    "Calling Node.js API crypto.randomFillSync failed",
    "Node.js ES modules are not directly supported, see https://docs.rs/getrandom#nodejs-es-module-support",
  ],
  "0.3": ["getrandom: this target is not supported", "errno: did not return a positive value", "unexpected situation"],
};

// What an `Error`'s `Debug` and `Display` read of its code: its OS's
// number, with the OS's words for it, `strerror`'s, where 0.1's and 0.2's
// unix build has some, the platform's table's (ADR 0462); else its
// internal code's words; else nothing, an unknown code.
function $getrandomErrorParts(code, version) {
  const os = $getrandomRawOsError(code, version);
  if (os !== undefined) return { os, words: version === "0.3" ? undefined : $getrandomOsWords(os) };
  const internal = code - $getrandomInternalStart[version];
  return { words: internal >= 0 ? $getrandomWords[version][internal] : undefined };
}
// `strerror_r`'s words for the OS's `errno`, as 0.1's and 0.2's unix build
// reads them: the platform's table's for a number it names, else none, as
// `strerror_r` fails, `EINVAL`, of a number it doesn't know.
function $getrandomOsWords(errno) {
  const process = $process("showing `getrandom::Error` of an OS error");
  const named = Object.values(process.getBuiltinModule("node:os").constants.errno).includes(errno);
  return named ? $osErrorOf(errno).message : undefined;
}

// Its `Display` into a writer, as its version writes it: the OS's words,
// `pad`'s of them, or `write!`'s pieces of `OS Error: {}`; an internal
// code's words, one `write_str`; or `Unknown Error: {}`'s pieces.
function $writeGetrandomError(f, code, version, options) {
  const { os, words } = $getrandomErrorParts(code, version);
  if (os !== undefined && words !== undefined) return $writePadded(f, words, options);
  if (os !== undefined) {
    f.write("OS Error: ");
    return $writeInteger(f, os);
  }
  if (words !== undefined) return f.write(words);
  f.write("Unknown Error: ");
  $writeInteger(f, code);
}
// Its `Debug` into a writer, by std's `debug_struct`, as its version's
// writes it: `Error { os_error, description }`, `Error { internal_code,
// description }` or `Error { unknown_code }`, each number given the
// `Formatter`'s options, each description a `str`'s `Debug`.
function $writeGetrandomErrorDebug(f, code, version, options = {}) {
  const { os, words } = $getrandomErrorParts(code, version);
  const number = (g, n) => $writeDebugInteger(g, n, n.toString(16), n.toString(16).toUpperCase(), options);
  const fields = [];
  if (os !== undefined) fields.push(["os_error", os, number]);
  else fields.push([words !== undefined ? "internal_code" : "unknown_code", code, number]);
  if (words !== undefined) fields.push(["description", words, $writeDebugStr]);
  $writeDebugStruct(f, "Error", fields, options.alternate);
}
// Its `Display` and `Debug` as text, written as into a writer.
function $getrandomErrorText(code, version, options, debug) {
  let text = "";
  const f = { write: (s) => void (text += s), writeChar: (c) => void (text += c), armed: false };
  if (debug) $writeGetrandomErrorDebug(f, code, version, options);
  else $writeGetrandomError(f, code, version, options);
  return text;
}
function $displayGetrandomError(code, version, options) {
  return $getrandomErrorText(code, version, options, false);
}
function $debugGetrandomError(code, version, options) {
  return $getrandomErrorText(code, version, options, true);
}

// One of `version`, of `TypeId` `type`, as a `dyn Error` (ADR 0141): shown
// as its version shows it; its `source()` none, as its `Error` impl's; its
// `type_id` `type` (ADR 0331).
function $getrandomErrorDyn(type, version) {
  return {
    Debug: () => ({
      fmt: (code, options) => $debugGetrandomError(code, version, options),
      fmtInto: (f, code, options) => $writeGetrandomErrorDebug(f, code, version, options),
    }),
    Display: () => ({
      fmt: (code, options) => $displayGetrandomError(code, version, options),
      fmtInto: (f, code, options) => $writeGetrandomError(f, code, version, options),
    }),
    source: () => undefined,
    typeId: () => type,
  };
}
