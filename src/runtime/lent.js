// A JS iterator lent as a `&mut` (ADR 0071): it steps `it`, and what takes
// from it and stops early, as `take` or a loop that breaks does, closes
// this, not `it`, which its lender goes on stepping. The one owner of what
// std's `&mut I` passes on (`core/src/iter/traits/iterator.rs:4233`,
// `double_ended.rs:437`, ADR 1092 §3), each `it`'s, the crate's own where
// `$iterator` was given it: its `next`, `size_hint`, `advance_by`, `nth`
// and `try_fold`; its `next_back`, `nth_back` and `try_rfold`; and its
// `len`. Its `fold` is `it`'s `try_fold`, never stopped, as `&mut I`'s
// `spec_fold` is for a sized `I` (#162), and its `rfold`, `count` and
// `last` std's defaults, of those, so none reaches `it`'s own, which would
// take it from its lender (#747, #749).
function $lent(it) {
  // Of an iterable that's no iterator, an array a caller gives, a JS
  // iterator of it. Never `Iterator.from` of one: it calls a `$Steps`'
  // `[Symbol.iterator]()`, which arms what's lent to close at its end, as
  // a consumer that takes it by value does, so its lender's would be
  // dropped at its first `None` (#1508).
  if (typeof it.next !== "function") it = Iterator.from(it);
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
  // Its `size_hint`, `it`'s, as std's `&mut I` passes it on (#745).
  lent.sizeHint = () => $sizeHint(it);
  // What it lends, whose facts are its own: `$slicing` gives it its
  // slice's steps, which it passes on (#745).
  lent.lends = it;
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
