// `it.try_fold(init, f)` into an `Option` (ADR 0458): `f` of what's made so
// far and each item in turn, till one gives `None`, which it gives, the rest
// left to step; else what the last gave. Of what could look like `None`,
// `Option<()>`'s, `try_for_each`'s, each `Some` boxed (ADRs 0051, 0539).
function $tryFoldSome(it, init, f) {
  let made = init;
  for (let step = it.next(); !step.done; step = it.next()) {
    const some = f(made, step.value);
    if (some == null) return undefined;
    made = $someValue(some);
  }
  return $some(made);
}
