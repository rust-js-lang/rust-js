// `it.size_hint()` of a generic iterator (ADR 0170): exact of an array, or of
// one stepping through an array, and std's `(0, None)` of a lazy one, which
// may be any length; Rust lets a hint be any bounds that hold.
function $sizeHint(it) {
  const left = Array.isArray(it) ? it.length : Array.isArray(it.items) ? it.items.length - it.at : undefined;
  return left === undefined ? [0, undefined] : [left, left];
}
