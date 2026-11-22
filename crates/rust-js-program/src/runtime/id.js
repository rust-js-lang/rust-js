// The number an object is known by in a key that holds it, as its derived
// `Hash` and `Eq` find a type equal only to itself, by itself (ADR 1015,
// amended): the same of one object each time, and another of each other
// object. `None`, `null` or `undefined`, is itself.
const $ids = new WeakMap();
let $lastId = 0;

function $id(object) {
  if (object == null) return object;
  let id = $ids.get(object);
  if (id === undefined) $ids.set(object, (id = ++$lastId));
  return id;
}
