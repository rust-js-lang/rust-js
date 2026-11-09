
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
    return "{" + Array.from(v, ([k, x]) => $debug(k) + ": " + $debug(x)).join(", ") + "}";
  }
  if (v instanceof Set) {
    return "{" + Array.from(v, (x) => $debug(x)).join(", ") + "}";
  }
  if (typeof v === "object" && v !== null) {
    return "{ " + Object.entries(v).map(([k, x]) => k + ": " + $debug(x)).join(", ") + " }";
  }
  return String(v);
}

// A `Debug` builder's `entries(items)` of what holds what drops, or of an
// iterator whose steps may panic: each shown by `show` as it's stepped,
// then dropped by `drop`, where its items have a destructor, and as a panic
// showing it unwinds, as std's `for entry in entries { self.entry(&entry) }`
// drops it; what's left of `items` closed where a step or a show throws,
// `$eachOf`'s (#1141, #1147, #1148).
function $entriesShown(items, show, drop) {
  const shown = [];
  $eachOf(items, (entry) => {
    shown.push(drop === undefined ? show(entry) : $unwinds(() => show(entry), () => drop(entry)));
    drop?.(entry);
  });
  return shown;
}
