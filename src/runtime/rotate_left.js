// `v.rotate_left(n)`: its first `n` items moved to its end. More than it
// has fails Rust's assertion, of `n` named `name` as std names it.
function $rotateLeft(v, n, name) {
  if (n > v.length) throw new Error(`assertion failed: ${name} <= self.len()`);
  const items = v.slice();
  for (let i = 0; i < items.length; i++) {
    v[i] = items[(i + n) % items.length];
  }
}
