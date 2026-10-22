
// A string's text, a U+FEFF at its start too, which `TextDecoder` would drop
// as a byte order mark, where serde_json keeps every character.
const $JSON_UTF8 = new TextDecoder("utf-8", { ignoreBOM: true });
const $JSON_POW10 = Array.from({ length: 309 }, (_, i) => Number(`1e${i}`));

// `significand * 10 + digit`, a BigInt once it's past 2^53.
function $jsonGrow(significand, digit) {
  if (typeof significand === "number") {
    const grown = significand * 10 + digit;
    if (grown <= Number.MAX_SAFE_INTEGER) return grown;
  }
  return BigInt(significand) * 10n + BigInt(digit);
}

function $jsonHexDigit(c) {
  if (c >= 48 && c <= 57) return c - 48;
  if (c >= 65 && c <= 70) return c - 55;
  if (c >= 97 && c <= 102) return c - 87;
  return -1;
}

// serde's `Unexpected` of a number, a float as serde_json writes one.
function $jsonUnexpectedNumber(n) {
  return n.kind === "f" ? `floating point \`${$jsonNumber(n.value)}\`` : `integer \`${n.value}\``;
}

function $jsonChar(s) {
  if ([...s].length === 1) return s;
  throw $jsonError(`invalid value: string ${$debugStr(s)}, expected a character`);
}

// serde's `Unexpected` of a value already read.
function $jsonUnexpectedContent({ type, value }) {
  if (type === "unit") return "null";
  if (type === "bool") return `boolean \`${value}\``;
  if (type === "num") return $jsonUnexpectedNumber(value);
  if (type === "str") return `string ${$debugStr(value)}`;
  return type === "seq" ? "sequence" : "map";
}

// The variant a name names, or `other`'s, as a derive's variant visitor finds it.
function $jsonVariantNamed(name, variants, other) {
  for (const names of variants) {
    if (typeof names === "string" ? names === name : names.includes(name)) return $jsonName(names);
  }
  if (other !== undefined) return other;
  throw variants.length === 0
    ? $jsonError(`unknown variant \`${name}\`, there are no variants`)
    : $jsonError(`unknown variant \`${name}\`, expected ${$jsonOneOf(variants)}`);
}

function $jsonName(names) {
  return typeof names === "string" ? names : names[0];
}

// The field a key names, or -1 to skip its value.
function $jsonField(key, fields, deny) {
  const i = fields.findIndex(([names]) => (typeof names === "string" ? names === key : names.includes(key)));
  if (i >= 0 || !deny) return i;
  throw fields.length === 0
    ? $jsonError(`unknown field \`${key}\`, there are no fields`)
    : $jsonError(`unknown field \`${key}\`, expected ${$jsonOneOf(fields.map(([names]) => names))}`);
}

// serde's `OneOf`, of each name and its aliases, which serde keeps sorted.
function $jsonOneOf(entries) {
  const byCodePoint = (a, b) => {
    const [x, y] = [[...a], [...b]];
    for (let i = 0; i < Math.min(x.length, y.length); i++) {
      const d = x[i].codePointAt(0) - y[i].codePointAt(0);
      if (d !== 0) return d;
    }
    return x.length - y.length;
  };
  const names = entries.flatMap((names) => (typeof names === "string" ? [names] : [...names].sort(byCodePoint)));
  const quoted = names.map((name) => `\`${name}\``);
  if (quoted.length === 1) return quoted[0];
  if (quoted.length === 2) return `${quoted[0]} or ${quoted[1]}`;
  return `one of ${quoted.join(", ")}`;
}

// What the crate reads with: `$json.u32`, `$json.option($json.string)`.
const $json = {
  bool: (json) => json.bool(),
  u8: (json) => json.int("u8", 0, 255),
  u16: (json) => json.int("u16", 0, 65535),
  u32: (json) => json.int("u32", 0, 4294967295),
  usize: (json) => json.int("usize", 0, 4294967295),
  i8: (json) => json.int("i8", -128, 127),
  i16: (json) => json.int("i16", -32768, 32767),
  i32: (json) => json.int("i32", -2147483648, 2147483647),
  isize: (json) => json.int("isize", -2147483648, 2147483647),
  u64: (json) => json.int("u64", 0n, 18446744073709551615n),
  i64: (json) => json.int("i64", -9223372036854775808n, 9223372036854775807n),
  f64: (json) => json.f64(),
  f32: (json) => json.f32(),
  string: (json) => json.string(),
  str: (json) => json.borrowedStr(),
  // serde_json's `Value` (ADR 0083), from whatever's there, and its `Number`.
  value: (json) => json.deserializeAny("any valid JSON value", $JSON_VALUE),
  number: (json) =>
    json.deserializeAny("a JSON number", {
      number: (n) => {
        if (n.kind === "f" && !Number.isFinite(n.value)) throw $jsonError("not a JSON number");
        return n;
      },
    }),
  char: (json) => json.char(),
  unit: (json) => json.unit(),
  option: (read) => (json) => json.option(read),
  // A generic `T` in an `Option`, boxed when it looks like `None` (ADR 0051).
  some: (read) => (json) => $some(read(json)),
  // A generic type's reader, given its type arguments' readers (ADR 0080).
  with:
    (deserialize, ...readers) =>
    (json) =>
      deserialize(json, ...readers),
  vec: (read) => (json) => json.vec(read),
  set: (read) => (json) => new Set(json.vec(read)),
  map: (readKey, read) => (json) => json.map(readKey, read),
  tuple: (...reads) => (json) => json.tuple(reads),
  // `#[serde(try_from = "T")]`: the value of a `TryFrom`'s `Ok`, and of an
  // `Err`, serde's `Error::custom` of its `display`.
  tried: (result, display) => {
    if (result.TAG === "Err") throw $jsonError(display(result._0));
    return result._0;
  },
  // serde's impl for `Result`: `{"Ok": ..}` or `{"Err": ..}`.
  result: (ok, err) => (json) =>
    json.enum("Result", ["Ok", "Err"], (variant, content) => ({
      TAG: variant,
      _0: content.newtype(variant === "Ok" ? ok : err),
    })),
  array: (length, read) => (json) => json.array(length, read),
  // An object's keys.
  key: {
    string: (key) => key.string(),
    char: (key) => key.char(),
    bool: (key) => key.bool(),
    number: (read) => (key) => key.number(read),
  },
};

// serde_json's `ValueVisitor`: a `Value` of each kind.
const $JSON_VALUE = {
  unit: () => "Null",
  bool: (b) => ({ TAG: "Bool", _0: b }),
  number: (n) => ({ TAG: "Number", _0: n }),
  string: (s) => ({ TAG: "String", _0: s }),
  seq: (seq) => {
    const items = [];
    while (seq.next()) items.push(seq.value($json.value));
    return { TAG: "Array", _0: items };
  },
  map: (map) => {
    const entries = new Map();
    while (map.next()) {
      const key = map.key().string();
      entries.set(key, map.value($json.value));
    }
    return { TAG: "Object", _0: entries };
  },
};

// `serde_json::from_str`: a `Result` of what `read` reads, which must be
// all there is.
function $fromJson(text, read) {
  const json = new $JsonReader(text);
  try {
    const value = read(json);
    json.end();
    return { TAG: "Ok", _0: value };
  } catch (e) {
    if (e instanceof $JsonError) return { TAG: "Err", _0: { message: e.message, line: e.line, column: e.column } };
    throw e;
  }
}
