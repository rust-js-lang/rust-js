// A JS iterator lent as a `&mut` (ADR 0071): it steps `it`, and what takes
// from it and stops early, as `take` or a loop that breaks does, closes
// this, not `it`, which its lender goes on stepping. Its `nth` is `it`'s,
// as std's `&mut I` gives it, the crate's own where `$iterator` was given
// it; its `fold`, std's, of `next` (#162), or of `it`'s `tryFold` where `it`
// is std's adapter of one, or a range, whose `tryFold` it is too.
function $lent(it) {
  // Its `nth`, `it`'s, as std's `&mut I` gives it, the crate's own where
  // `$iterator` was given it; its `advance_by`, `it`'s own (ADR 0913); its
  // `fold` that `tryFold`, as std's `&mut I` folds by its `I::try_fold`: an
  // inclusive range's ends it exhausted at its last (#414). And its back
  // and its length, `it`'s, as std's `&mut I` passes on an
  // `I: DoubleEndedIterator`'s `next_back`, `nth_back` and `try_rfold` and
  // an `ExactSizeIterator`'s `len`; not its `rfold`, which it folds by that
  // `try_rfold`, as its dictionary does (ADRs 0928, 0982).
  const lent = $forward(new $Steps(() => it.next()), it, {
    nth: "nth",
    advanceBy: Object.hasOwn(it, "advanceBy") ? "advanceBy" : undefined,
    tryFold: "tryFold",
    fold: it.tryFold === undefined ? undefined : (init, f, unwind) => it.tryFold(init, f, () => false, unwind),
    nthBack: "nthBack",
    tryRfold: "tryRfold",
  });
  if (it.back !== undefined) lent.back = () => it.back();
  if (it.len !== undefined) lent.len = () => it.len();
  // Of std's struct, compiled (ADR 0871): its fields `it`'s, read and
  // written through it, so its compiled bodies step `it` where it is, as
  // std's `&mut I` steps its `I`; what stops early closes `lent`, not it.
  if (it.$steps !== undefined) {
    // Its own steps first, then `it`'s fields, `iter` and `count$`, which
    // a helper's name never hides (ADR 0912).
    const of = (target, key) => (Object.hasOwn(target, key) || !Object.hasOwn(it, key) ? target : it);
    return new Proxy(lent, {
      get: (target, key) => of(target, key)[key],
      set: (target, key, value) => {
        of(target, key)[key] = value;
        return true;
      },
    });
  }
  return lent;
}
