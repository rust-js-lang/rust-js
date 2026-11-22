// `js::SymbolFor::from_value(value)` (ADR 0634): the value where it's the
// registry's symbol, `undefined` where it's any other.
function $symbolFor(value) {
  return typeof value === "symbol" && Symbol.keyFor(value) !== undefined ? value : undefined;
}
