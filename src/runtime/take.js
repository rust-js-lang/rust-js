// `mem::replace(r, v)` and `mem::take(r)` of an object a `&mut` is (ADR
// 0147): what it was, a copy, as `r` itself becomes `v` in place.
function $take(target, value) {
  const old = $apart(target);
  $assign(target, value);
  return old;
}
