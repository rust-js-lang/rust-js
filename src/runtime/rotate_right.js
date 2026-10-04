// `v.rotate_right(n)`: its last `n` items moved to its start. More than it
// has fails Rust's assertion, of `n` named `name` as std names it.
function $rotateRight(v, n, name) {
  if (n > v.length) throw new Error(`assertion failed: ${name} <= self.len()`);
  const items = v.slice();
  for (let i = 0; i < items.length; i++) {
    v[(i + n) % items.length] = items[i];
  }
}
