
// Past its end, `item`, which Rust's `insert` took, is dropped by `drop`
// as the panic unwinds, then the panic, `message` or `Vec`'s (#1171).
function $insertAt(v, index, item, drop, message) {
  if (index > v.length) {
    drop?.(item);
    throw new Error(message ?? `insertion index (is ${index}) should be <= len (is ${v.length})`);
  }
  v.splice(index, 0, item);
}
