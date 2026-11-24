// A box's copy of an object generic code is given a `&mut` of (ADR 0074):
// its top level apart, a new array, `Map`, `Set` or object of the same
// items, which what the code moves out of the box is, where the object,
// written back after, becomes what the box holds then.
function $apart(value) {
  if (Array.isArray(value)) return value.slice();
  // Of keys found by their own `Hash`, one of its class, which finds them
  // so, each under the bytes it's held by, asking no key's `hash` (#2548).
  if (typeof value?.heldBytes === "function") return value.constructor.ofHashed(value, value);
  if (value instanceof Map) return new Map(value);
  if (value instanceof Set) return new Set(value);
  // std's compiled struct, whose methods step itself: another of its
  // fields (#537).
  if (value?.$copy !== undefined) return value.$copy();
  // An instance's prototype kept, a hasher's methods.
  if (value !== null && typeof value === "object") return Object.assign(Object.create(Object.getPrototypeOf(value)), value);
  return value;
}
