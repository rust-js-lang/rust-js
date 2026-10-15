// A map, or a set, of `Cow`s, each kept under what it borrows, as std's
// `Hash` and `Eq` of one deref it (ADR 0402): `Borrowed("a")`, `Owned("a")`
// and a `&str` "a" alike. What it borrows, text or an array, is never an
// object like a `Cow`'s, `{ TAG, _0 }`, so either is known by its shape.
function $cowKey(key) {
  return $key(typeof key === "object" && !Array.isArray(key) ? key._0 : key);
}

class $CowMap extends $KeyMap {
  keyOf(key) {
    return $cowKey(key);
  }
}

class $CowSet extends $KeySet {
  keyOf(key) {
    return $cowKey(key);
  }
}
