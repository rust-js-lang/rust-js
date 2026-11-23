// serde_path_to_error, a crate rustc builds whose API rust-js writes (ADR
// 1188): `deserialize(&mut serde_json::Deserializer::from_str(text))`, what
// `read` reads at the reader's place, not to the text's end, as the crate's
// `deserialize` reads it. Its error is the crate's `Error`, `{ path, original
// }`: where in the value the reader was as the first mistake was made, the
// crate's `Track`, then serde_json's error. A path is its segments: an item's
// index, a number; a key's text, a string; a variant's name, `{ variant }`;
// a key that isn't text, `null`, `?`. The reader keeps them as it reads
// (`$JsonSeq`, `$JsonMap`, `$JsonKey` and `$JsonReader`'s `enum`), and each
// `$JsonError` made meanwhile keeps a copy; a plain `serde_json::from_str`
// keeps none.
function $jsonPathRead(json, read) {
  const outer = $JsonError.chain;
  json.chain = $JsonError.chain = [];
  try {
    return { TAG: "Ok", _0: read(json) };
  } catch (e) {
    if (!(e instanceof $JsonError)) throw e;
    return { TAG: "Err", _0: { path: e.path ?? [], original: $jsonErrorOf(e) } };
  } finally {
    json.chain = undefined;
    json.keying = false;
    $JsonError.chain = outer;
  }
}

// A `Path`'s `Display` into a writer, as the crate writes it: `.` of none;
// else each segment, a key's or a variant's text `write!`'s `{}`, an index
// `[{}]`'s pieces, a key that isn't text `?`, each but an index after its
// separator, `""` of the first and `.` after.
function $writeJsonPath(f, path) {
  if (path.length === 0) return f.write(".");
  let separator = "";
  for (const segment of path) {
    if (typeof segment === "number") {
      f.write("[");
      $writeInteger(f, segment);
      f.write("]");
    } else {
      f.write(separator);
      f.write(segment === null ? "?" : (segment.variant ?? segment));
    }
    separator = ".";
  }
}
// Its `Error`'s `Display` into a writer: its path's and `: `, unless the
// path is none or only keys that aren't text, then serde_json's error's.
// Neither reads the `Formatter`'s options, as `write!` doesn't.
function $writeJsonPathError(f, e) {
  if (!e.path.every((segment) => segment === null)) {
    $writeJsonPath(f, e.path);
    f.write(": ");
  }
  $writeJsonError(f, e.original);
}
// Each's derived `Debug` into a writer: `Error { path, original }`, the
// path `Path { segments }`, each segment `Seq { index }`, `Map { key }`,
// `Enum { variant }` or `Unknown`, an index given the `Formatter`'s options,
// serde_json's error its own `Debug`.
function $writeDebugJsonPath(f, path, options = {}) {
  const pretty = options.alternate;
  const index = (g, n) => $writeDebugInteger(g, n, n.toString(16), n.toString(16).toUpperCase(), options);
  const segment = (g, s) => {
    if (s === null) return g.write("Unknown");
    if (typeof s === "number") return $writeDebugStruct(g, "Seq", [["index", s, index]], pretty);
    if (typeof s === "string") return $writeDebugStruct(g, "Map", [["key", s, $writeDebugStr]], pretty);
    $writeDebugStruct(g, "Enum", [["variant", s.variant, $writeDebugStr]], pretty);
  };
  const segments = (g, items) => $writeDebugList(g, items, segment, pretty);
  $writeDebugStruct(f, "Path", [["segments", path, segments]], pretty);
}
function $writeDebugJsonPathError(f, e, options = {}) {
  const path = (g, p) => $writeDebugJsonPath(g, p, options);
  const fields = [["path", e.path, path], ["original", e.original, $writeDebugJsonError]];
  $writeDebugStruct(f, "Error", fields, options.alternate);
}
// Each as text, written as into a writer.
function $jsonPathText(write, ...args) {
  let text = "";
  const f = { write: (s) => void (text += s), writeChar: (c) => void (text += c), armed: false };
  write(f, ...args);
  return text;
}
function $displayJsonPath(path) {
  return $jsonPathText($writeJsonPath, path);
}
function $displayJsonPathError(e) {
  return $jsonPathText($writeJsonPathError, e);
}
function $debugJsonPath(path, options) {
  return $jsonPathText($writeDebugJsonPath, path, options);
}
function $debugJsonPathError(e, options) {
  return $jsonPathText($writeDebugJsonPathError, e, options);
}

// Its `Error`, of `TypeId` `type`, as a `dyn Error` (ADR 0141): shown as the
// crate shows it; its `source()` serde_json's error's, as its `Error` impl's
// is its `original`'s, which has none; its `type_id` `type` (ADR 0331).
function $jsonPathErrorDyn(type) {
  return {
    Debug: () => ({
      fmt: (e, options) => $debugJsonPathError(e, options),
      fmtInto: (f, e, options) => $writeDebugJsonPathError(f, e, options),
    }),
    Display: () => ({ fmt: $displayJsonPathError, fmtInto: (f, e) => $writeJsonPathError(f, e) }),
    source: () => undefined,
    typeId: () => type,
  };
}
