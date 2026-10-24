// `it.try_fold(init, f)` into a `Result` (ADR 0458): `f` of what's made so
// far and each item in turn, till one gives an `Err`, which it gives, the
// rest left to step; else the `Ok` the last gave.
// It's `it`'s `tryFold`, as std's adapters route it.
function $tryFoldOk(it, init, f) {
  return $lent(it).tryFold({ TAG: "Ok", _0: init }, (made, x) => f(made._0, x), (made) => made.TAG === "Err");
}
