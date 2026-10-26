
// Two values `==` compares part by part, each part that's a `RefCell`
// borrowed to compare it, as are `a` and `b` where they're `cells`. One
// that's the object it holds is also the value its guard reads, which isn't
// borrowed to be compared (ADR 0863).
function $eq(a, b, cells) {
  // Not an object's `===` of itself: one of a NaN isn't equal to itself.
  if ((a === b && typeof a !== "object") || (a == null && b == null)) {
    return true;
  }
  if (typeof a !== "object" || typeof b !== "object" || a === null || b === null) {
    return false;
  }
  // A `RefCell` mutably borrowed can't be borrowed to compare (ADR 0328).
  if (cells && (a.borrows < 0 || b.borrows < 0)) throw new Error("RefCell already mutably borrowed");
  if (Array.isArray(a)) {
    return Array.isArray(b) && a.length === b.length && a.every((x, i) => $eq(x, b[i], true));
  }
  // A map's or a set's, held in what's compared (ADR 0059).
  if (a instanceof Set) return b instanceof Set && $setEq(a, b);
  if (a instanceof Map) return b instanceof Map && $mapEq(a, b, (x, y) => $eq(x, y, true));
  // A key one has and the other hasn't is a `None` the other left out (ADR 0280).
  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
  return Array.from(keys).every((k) => $eq(a[k], b[k], true));
}
// Two maps (ADR 0059): as many entries, and each key of `a` one of `b`'s,
// whose value `eq` says is equal, `===` where it's given none; and two sets,
// each key of `a` one of `b`'s. A map keyed by a struct, `$KeyMap`, looks
// its keys up by value.
function $mapEq(a, b, eq = (x, y) => x === y) {
  if (a.size !== b.size) return false;
  for (const [key, value] of a) {
    if (!b.has(key) || !eq(value, b.get(key))) return false;
  }
  return true;
}
function $setEq(a, b) {
  if (a.size !== b.size) return false;
  for (const key of a) {
    if (!b.has(key)) return false;
  }
  return true;
}
