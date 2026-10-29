// `it.reduce(f)`: its first item, folded with each after it by `f`, or
// `None` of none, where JS's `reduce` throws, closed, as std's `next()?`
// drops it (#242); boxed where it may look like `None` (ADRs 0051, 0479).
// Given `unwind`, what drops what it folded where a stage throws, a panic,
// passed to its fold (#423); and closed where its first step throws.
function $reduce(it, f, boxed = false, unwind = undefined) {
  let first;
  try {
    first = it.next();
  } catch (error) {
    it.return?.();
    throw error;
  }
  if (first.done) {
    it.return?.();
    return undefined;
  }
  const value = $foldDropping(it, first.value, f, unwind);
  return boxed ? $some(value) : value;
}
