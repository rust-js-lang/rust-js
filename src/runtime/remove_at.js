// `Vec::remove(index)` past its end: std's panic, its message as std's.
// The removal itself is JS's `splice` where it's called (#1692).
function $removeOutOfBounds(v, index) {
  throw new Error(`removal index (is ${index}) should be < len (is ${v.length})`);
}
