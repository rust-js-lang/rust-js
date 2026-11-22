
// Past its end, `item`, which Rust's `insert` took, is dropped by `drop`
// as the panic unwinds, then the panic, `message` or `Vec`'s (#1171).
function $insertAt(v, index, item, drop, message) {
  if (index > v.length) {
    drop?.(item);
    // `Vec`'s, formatted, a `String`; a `VecDeque`'s `message`, its
    // `assert!`'s literal, a `&str` (ADR 0660, amended).
    throw message === undefined ? $stringPanic(`insertion index (is ${index}) should be <= len (is ${v.length})`) : new Error(message);
  }
  v.splice(index, 0, item);
}
