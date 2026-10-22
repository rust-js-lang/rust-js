// A box's copy of an object generic code is given a `&mut` of (ADR 0074):
// its top level apart, a new array, `Map`, `Set` or object of the same
// items, which what the code moves out of the box is, where the object,
// written back after, becomes what the box holds then.
function $apart(value) {
  if (Array.isArray(value)) return value.slice();
  if (value instanceof Map) return new Map(value);
  if (value instanceof Set) return new Set(value);
  // An instance's prototype kept, a hasher's methods.
  if (value !== null && typeof value === "object") return Object.assign(Object.create(Object.getPrototypeOf(value)), value);
  return value;
}
