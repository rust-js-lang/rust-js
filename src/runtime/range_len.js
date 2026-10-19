// How many numbers an exclusive range, `a..b`, has left: an
// `ExactSizeIterator`'s `len()`, of its state (ADR 0456).
function $rangeLen(range) {
  return range.start < range.end ? range.end - range.start : 0;
}
