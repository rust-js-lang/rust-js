// `it.rev()` of a JS iterator that knows its back, a `$iter` or one closed
// to drop what it holds: it, stepped from its back, as Rust's `Rev` steps
// what it holds by `next_back`; its back its front, its length its own, and
// closed, `return()`, as it is, which drops what it hasn't given in its
// order, front to back (ADR 0516); its size hint its own, as std's `Rev`'s
// is (ADR 0989). Its `fold`, `nth` and `try_fold` are
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
  rev.hint = () => $sizeHint(it);
  // Its `fold`, `nth` and `try_fold`, its source's `rfold`, `nth_back` and
  // `try_rfold`, as std's `Rev`'s are; its `nth_back` and `advance_back_by`
  // its source's `nth` and `advance_by` where they drop what they pass over
  // themselves, a `Vec`'s `IntoIter`'s, from either end (#234, #250).
  $forward(rev, it, {
    fold: "rfold",
    nth: "nthBack",
    tryFold: "tryRfold",
    nthBack: it.ownNth ? "nth" : undefined,
    advanceBackBy: it.ownNth ? "advanceBy" : undefined,
    // Its `advance_by`, its source's `advance_back_by`, std's, of its
    // `next_back`, each dropped, `drop`: what its `nth` steps past.
    advanceBy: (m, drop) => {
      // One that drops what it passes over itself, a `Vec`'s, as one slice
      // (#250).
      if (it.advanceBackBy !== undefined) return it.advanceBackBy(m);
      for (; m > 0; m--) {
        const step = it.back();
        if (step.done) break;
        drop?.(step.value);
      }
      return m;
    },
  });
  if (it.dropItem !== undefined) rev.dropItem = it.dropItem;
  return rev;
}
