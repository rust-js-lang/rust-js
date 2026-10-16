// A `Vec`'s capacity (ADR 0419), which JS's array hasn't: kept where a
// crate asks it, as std promises it, at least its length and what it was
// given, and never less where it's shortened. Its own number, not native
// Rust's growth's.
const $capacities = new WeakMap();

function $capacity(v) {
  return Math.max(v.length, $capacities.get(v) ?? 0);
}

// `Vec::with_capacity(n)`: none yet, room for `n`.
function $withCapacity(n) {
  const v = [];
  $capacities.set(v, n);
  return v;
}

// `v.reserve(n)`, `reserve_exact(n)`: room for `n` more.
function $reserve(v, additional) {
  $capacities.set(v, Math.max($capacity(v), v.length + additional));
}

// `v.shrink_to(n)`, `shrink_to_fit()` of `0`: no more than `n`, but its
// length.
function $shrinkTo(v, min) {
  $capacities.set(v, Math.max(v.length, Math.min($capacity(v), min)));
}

// Before it's shortened, `clear()`, `pop()` and the rest: what it has, kept.
function $keepCapacity(v) {
  $capacities.set(v, $capacity(v));
}
