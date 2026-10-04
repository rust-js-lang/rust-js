
function $debug(v) {
  // Escaped as Rust's `{:?}` escapes it, `"\u{1b}"`, not as JSON does.
  if (typeof v === "string") {
    return $debugStr(v);
  }
  if (Array.isArray(v)) {
    return "[" + v.map($debug).join(", ") + "]";
  }
  if (v === undefined) {
    return "()";
  }
  // A `HashMap` and a `HashSet` (ADR 0059), in braces as Rust shows them.
  if (v instanceof Map) {
    return "{" + [...v].map(([k, x]) => $debug(k) + ": " + $debug(x)).join(", ") + "}";
  }
  if (v instanceof Set) {
    return "{" + [...v].map($debug).join(", ") + "}";
  }
  if (typeof v === "object" && v !== null) {
    return "{ " + Object.entries(v).map(([k, x]) => k + ": " + $debug(x)).join(", ") + " }";
  }
  return String(v);
}
