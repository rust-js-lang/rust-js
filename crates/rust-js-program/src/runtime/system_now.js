// `SystemTime::now()`: the nanoseconds since the Unix epoch, of JS's
// clock's milliseconds (ADR 0447).
function $systemNow() {
  return BigInt(Date.now()) * 1000000n;
}
