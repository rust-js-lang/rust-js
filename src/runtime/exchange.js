// `mem::swap(a, b)` of two objects `&mut`s are (ADR 0147): each becomes what
// the other was, in place.
function $exchange(a, b) {
  $assign(b, $take(a, b));
}
