// `v.swap(a, b)`: out of bounds, a slice's message of the first index std
// checks, or a `VecDeque`'s assertion (`deque`).
function $swap(v, a, b, raisedAt, deque = false) {
  if (deque && a >= v.length) throw $panic("assertion failed: i < self.len()", $stdSites.vecDequeSwap[0]);
  if (deque && b >= v.length) throw $panic("assertion failed: j < self.len()", $stdSites.vecDequeSwap[1]);
  if (a >= v.length || b >= v.length) {
    throw $stringPanic(`index out of bounds: the len is ${v.length} but the index is ${a >= v.length ? a : b}`, raisedAt);
  }
  [v[a], v[b]] = [v[b], v[a]];
}
