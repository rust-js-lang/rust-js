// `a.duration_since(b)` of two `SystemTime`s: `Ok` of the time from `b`
// to `a`, or `Err` of a `SystemTimeError` of the time `b` is after (ADR
// 0447).
function $timeSince(a, b) {
  return a >= b ? { TAG: "Ok", _0: a - b } : { TAG: "Err", _0: b - a };
}
