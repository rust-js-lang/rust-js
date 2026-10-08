
function $key(value) {
  if (value == null) {
    return "~";
  }
  switch (typeof value) {
    case "string":
      return JSON.stringify(value);
    case "bigint":
      return value + "n";
    case "object":
      if (Array.isArray(value)) {
        return "[" + value.map($key).join(",") + "]";
      }
      // A field that's `None`, there or left out, is no part of it (ADR 0280).
      const keys = Object.keys(value).filter((k) => value[k] != null);
      return "{" + keys.sort().map((k) => JSON.stringify(k) + ":" + $key(value[k])).join(",") + "}";
    default:
      return String(value);
  }
}
