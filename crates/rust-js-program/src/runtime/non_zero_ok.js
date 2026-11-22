// A `NonZero`'s `parse` or `try_from`: its number's, then an error, `zero`,
// where that's 0, as std's `NonZero::new(..)?` is (ADR 0177).
function $nonZeroOk(result, zero) {
  return result.TAG === "Ok" && result._0 == 0 ? { TAG: "Err", _0: zero } : result;
}
