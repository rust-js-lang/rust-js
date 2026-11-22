// `Instant::now()`: nanoseconds of a clock that only goes forward, JS's
// `performance`, from when the page or process began, past the epoch as
// a platform's own begins past its boot (ADR 0447).
function $instantNow() {
  return BigInt(Math.round((performance.timeOrigin + performance.now()) * 1e6));
}
