// `a.duration_since(b)` of two `Instant`s, and `a - b`: the time from `b`
// to `a`, or zero where `b` is after, as std's saturates (ADR 0447).
function $instantSince(a, b) {
  return a > b ? a - b : 0n;
}
