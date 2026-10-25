// `it.reduce(f)`: its first item, folded with each after it by `f`, or
// `None` of none, where JS's `reduce` throws, closed, as std's `next()?`
// drops it (#242); boxed where it may look like `None` (ADRs 0051, 0479).
function $reduce(it, f, boxed = false) {
  const first = it.next();
  if (first.done) {
    it.return?.();
    return undefined;
  }
  const value = it.reduce(f, first.value);
  return boxed ? $some(value) : value;
}
