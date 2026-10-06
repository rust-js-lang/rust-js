// A dictionary's value of `key` (ADR 0225): `Some` of it, boxed where it
// looks like `None`, a JSON `null`'s `Some(None)`; `None` where the key
// isn't its own, as `toString` isn't.
function $dictGet(dict, key) {
  return Object.hasOwn(dict, key) ? $some(dict[key]) : undefined;
}
