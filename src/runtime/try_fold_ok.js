// `it.try_fold(init, f)` into a `Result` (ADR 0458): `f` of what's made so
// far and each item in turn, till one gives an `Err`, which it gives, the
// rest left to step; else the `Ok` the last gave.
function $tryFoldOk(it, init, f) {
  let made = { TAG: "Ok", _0: init };
  for (let step = it.next(); !step.done; step = it.next()) {
    made = f(made._0, step.value);
    if (made.TAG === "Err") return made;
  }
  return made;
}
