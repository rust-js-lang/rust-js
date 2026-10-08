
function $eq(a, b) {
  if (a === b || (a == null && b == null)) {
    return true;
  }
  if (typeof a !== "object" || typeof b !== "object" || a === null || b === null) {
    return false;
  }
  if (Array.isArray(a)) {
    return Array.isArray(b) && a.length === b.length && a.every((x, i) => $eq(x, b[i]));
  }
  // A key one has and the other hasn't is a `None` the other left out (ADR 0280).
  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
  return [...keys].every((k) => $eq(a[k], b[k]));
}
