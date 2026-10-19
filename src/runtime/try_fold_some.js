// `it.try_fold(init, f)` into an `Option` (ADR 0458): `f` of what's made so
// far and each item in turn, till one gives `None`, which it gives, the rest
// left to step; else what the last gave.
function $tryFoldSome(it, init, f) {
  let made = init;
  for (let step = it.next(); !step.done; step = it.next()) {
    made = f(made, step.value);
    if (made == null) return made;
  }
  return made;
}
