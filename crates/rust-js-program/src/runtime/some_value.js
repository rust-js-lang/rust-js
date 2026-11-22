// What's in a `$some`: the box one level shallower, or the `null` it kept.
function $someValue(x) {
  if (x != null && typeof x === "object" && "$someNone" in x) {
    return x.$someNone === 0 ? x.$value : { ...x, $someNone: x.$someNone - 1 };
  }
  return x;
}
