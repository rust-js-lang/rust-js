// `it.try_fold(init, f)` into an `Option` (ADR 0458): `f` of what's made so
// far and each item in turn, till one gives `None`, which it gives, the rest
// left to step; else what the last gave. Of what could look like `None`,
// `Option<()>`'s, `try_for_each`'s, each `Some` boxed (ADRs 0051, 0539).
// It's `it`'s `tryFold`, as std's adapters route it.
function $tryFoldSome(it, init, f) {
  return $lent(it).tryFold($some(init), (made, x) => f($someValue(made), x), (made) => made == null);
}
