// Std's `Try`, as std's own generic code is given it (ADR 0913): an
// `R: Try`'s dictionary, `{ fromOutput, branch, FromResidual }`, `branch`
// giving the `ControlFlow` `?` reads, `FromResidual` its supertrait's,
// itself.

// A `ControlFlow`'s: its `Continue`'s value is its output, and a `Break` is
// its residual as it is.
const $controlFlowTry = {
  fromOutput: (output) => ({ TAG: "Continue", _0: output }),
  branch: (flow) => (flow.TAG === "Continue" ? flow : { TAG: "Break", _0: flow }),
  fromResidual: (flow) => flow,
  FromResidual: () => $controlFlowTry,
};

// A `NeverShortCircuit`'s, which is its value (ADR 0584): never a break.
const $neverShortCircuitTry = {
  fromOutput: (output) => output,
  branch: (output) => ({ TAG: "Continue", _0: output }),
  fromResidual: (never) => never,
  FromResidual: () => $neverShortCircuitTry,
};

// An `Option`'s, as generic code holds it (ADR 0051): `Some(x)` is
// `$some(x)`, boxed where `x` looks like `None`; its residual, `None`,
// `undefined` (#1864).
const $optionTry = {
  fromOutput: (output) => $some(output),
  branch: (o) => (o != null ? { TAG: "Continue", _0: $someValue(o) } : { TAG: "Break", _0: undefined }),
  fromResidual: () => undefined,
  FromResidual: () => $optionTry,
};

// A `Result`'s: its `Ok`'s value is its output, and an `Err` its residual
// as it is, which std's generic code gives back to the same `R`, by
// `From<E> for E` (#1864).
const $resultTry = {
  fromOutput: (output) => ({ TAG: "Ok", _0: output }),
  branch: (r) => (r.TAG === "Ok" ? { TAG: "Continue", _0: r._0 } : { TAG: "Break", _0: r }),
  fromResidual: (r) => r,
  FromResidual: () => $resultTry,
};

// What a JS iterator's `tryFold(init, f, broke)` folds into, any `R`, as a
// `Try`: what it's made so far is its output, and a value `broke` says
// stops it its residual, as it is, so std's compiled `try_fold` given this
// folds as that `tryFold` does.
function $tryOf(broke) {
  const tried = {
    fromOutput: (made) => made,
    branch: (made) => (broke(made) ? { TAG: "Break", _0: made } : { TAG: "Continue", _0: made }),
    fromResidual: (made) => made,
    FromResidual: () => tried,
  };
  return tried;
}

// `it.try_fold(init, f)` into an `R` of its `Try`, `tried`: `f` of what's
// made so far and each item in turn, till what it gives breaks, which it
// gives, the rest left to step; else what the last gave. It's `it`'s
// `tryFold`, as std's adapters route it, in `R`'s terms; given `ends`,
// what drops with `f`, dropped at its end, `$endsAfter`'s (#768).
function $tryFoldInto(it, init, f, tried, unwind, ends) {
  return $endsAfter(ends, (once) =>
    $lent(it).tryFold(
      tried.fromOutput(init),
      (made, x) => f(tried.branch(made)._0, x),
      (made) => tried.branch(made).TAG === "Break",
      unwind && ((made) => unwind(tried.branch(made)._0)),
      once,
    ),
  );
}
