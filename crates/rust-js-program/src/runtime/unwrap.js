
// `unwrap()`'s panic, its payload a `&str`, or `expect(message)`'s, its
// payload a `String`, as std's `expect_failed` shows it (ADR 0660, amended).
// Each is raised where it was called, `raisedAt` (#2292).
function $unwrap(value, raisedAt, message) {
  if (value == null) {
    throw message === undefined
      ? $panic("called `Option::unwrap()` on a `None` value", raisedAt)
      : $stringPanic(message, raisedAt);
  }
  return value;
}
