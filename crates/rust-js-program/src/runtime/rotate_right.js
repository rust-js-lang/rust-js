// `v.rotate_right(n)`: its last `n` items moved to its start. More than it
// has fails Rust's assertion, of `n` named `name` as std names it.
function $rotateRight(v, n, name) {
  if (n > v.length) {
    // A `VecDeque`'s, its `n`, or a slice's.
    const at = name === "n" ? $stdSites.vecDequeRotateRight[0] : $stdSites.rotateRight[0];
    throw $panic(`assertion failed: ${name} <= self.len()`, at);
  }
  const items = v.slice();
  for (let i = 0; i < items.length; i++) {
    v[(i + n) % items.length] = items[i];
  }
}
