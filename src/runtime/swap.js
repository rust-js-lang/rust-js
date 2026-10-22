// `v.swap(a, b)`: out of bounds, a slice's message of the first index std
// checks, or a `VecDeque`'s assertion (`deque`).
function $swap(v, a, b, deque = false) {
  if (deque && a >= v.length) throw new Error("assertion failed: i < self.len()");
  if (deque && b >= v.length) throw new Error("assertion failed: j < self.len()");
  if (a >= v.length || b >= v.length) {
    throw new Error(`index out of bounds: the len is ${v.length} but the index is ${a >= v.length ? a : b}`);
  }
  [v[a], v[b]] = [v[b], v[a]];
}
