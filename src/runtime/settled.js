// What a future resolves to, as a promise is resolved by JS's rules: one
// that's itself a thenable, a promise or an object with a `then`, a
// promise would adopt, where Rust's future gives it as it is. Boxed, then,
// `$settled`, and taken out of its box where it's awaited, `$unsettled`.
function $settled(value) {
  const thenable = value !== null && (typeof value === "object" || typeof value === "function") && typeof value.then === "function";
  return thenable ? { $settled: value } : value;
}
function $unsettled(value) {
  return value !== null && typeof value === "object" && "$settled" in value ? value.$settled : value;
}
