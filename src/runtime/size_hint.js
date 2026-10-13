// `it.size_hint()` of a generic iterator (ADR 0170): exact of one stepping
// through an array, `$iter`'s, which is how an array is given as one, and
// std's `(0, None)` of a lazy one, which may be any length; Rust lets a hint
// be any bounds that hold.
function $sizeHint(it) {
  const left = Array.isArray(it.items) ? it.end - it.at : undefined;
  return left === undefined ? [0, undefined] : [left, left];
}
