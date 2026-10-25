// `it.rev()` of a JS iterator that knows its back, a `$iter` or one closed
// to drop what it holds: it, stepped from its back, as Rust's `Rev` steps
// what it holds by `next_back`; its back its front, its length its own, and
// closed, `return()`, as it is, which drops what it hasn't given in its
// order, front to back (ADR 0516). Its `fold`, `nth` and `try_fold` are
// std's `Rev`'s, its source's `rfold`, `nth_back` and `try_rfold`, where
// they reach the crate's own (#220).
function $rev(it) {
  const rev = new $Steps(
    () => it.back(),
    () => it.return?.(),
  );
  rev.back = () => it.next();
  rev.from = [it];
  rev.len = () => it.len();
  // Its `advance_by`, its source's `advance_back_by`, std's, of its
  // `next_back`, each dropped, `drop`: what its `nth` steps past.
  rev.advanceBy = (m, drop) => {
    // One that drops what it passes over itself, a `Vec`'s, as one slice
    // (#250).
    if (it.advanceBackBy !== undefined) return it.advanceBackBy(m);
    for (; m > 0; m--) {
      const step = it.back();
      if (step.done) break;
      drop?.(step.value);
    }
    return m;
  };
  if (it.rfold !== undefined) rev.fold = (init, f) => it.rfold(init, f);
  if (it.nthBack !== undefined) rev.nth = (k) => it.nthBack(k);
  // One that drops what it passes over itself, a `Vec`'s `IntoIter`'s, from
  // either end (#234, #250).
  if (it.ownNthBack) rev.ownNth = true;
  if (it.ownNth) {
    rev.nthBack = (k) => it.nth(k);
    rev.advanceBackBy = (m) => it.advanceBy(m);
    rev.ownNthBack = true;
  }
  if (it.tryRfold !== undefined) rev.tryFold = (init, f, broke) => it.tryRfold(init, f, broke);
  return rev;
}
