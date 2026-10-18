// `a.checked_duration_since(b)` of two `Instant`s: the time from `b` to
// `a`, or `None` where `b` is after (ADR 0447).
function $instantCheckedSince(a, b) {
  return a >= b ? a - b : undefined;
}
