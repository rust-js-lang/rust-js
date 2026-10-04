// `*r = v` of an object a `&mut` is (ADR 0147): it becomes `v` in place, so
// each name for it sees `v`. An array its items, a `Map` or a `Set` its
// entries, an object its fields, those `v` hasn't gone, as another
// variant's are.
function $assign(target, value) {
  if (Array.isArray(target)) {
    target.length = value.length;
    for (let i = 0; i < value.length; i++) target[i] = value[i];
  } else if (target instanceof Map) {
    target.clear();
    for (const [key, item] of value) target.set(key, item);
  } else if (target instanceof Set) {
    target.clear();
    for (const item of value) target.add(item);
  } else {
    for (const key of Object.keys(target)) if (!(key in value)) delete target[key];
    Object.assign(target, value);
  }
}
