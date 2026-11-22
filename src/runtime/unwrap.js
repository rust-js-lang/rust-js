
// `unwrap()`'s panic, its payload a `&str`, or `expect(message)`'s, its
// payload a `String`, as std's `expect_failed` shows it (ADR 0660, amended).
function $unwrap(value, message) {
  if (value == null) {
    throw message === undefined ? new Error("called `Option::unwrap()` on a `None` value") : $stringPanic(message);
  }
  return value;
}
