// `it.reduce(f)`: its first item, folded with each after it by `f`, or
// `None` of none, where JS's `reduce` throws; boxed where it may look like
// `None` (ADRs 0051, 0479).
function $reduce(it, f, boxed = false) {
  const first = it.next();
  if (first.done) {
    return undefined;
  }
  const value = it.reduce(f, first.value);
  return boxed ? $some(value) : value;
}
