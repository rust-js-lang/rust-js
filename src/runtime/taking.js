// `it.take(n)` explicit, Rust's `Take` (ADR 0542): what it steps, `iter`,
// and how many it has left to take, `n`, kept, which a clone reads. Its
// back is the last of those it has left, the ones past it dropped by
// `drop` where items have a destructor, as Rust's `next_back` steps its
// source's `nth_back`; its length the fewer of its source's and `n`.
function $taking(iter, n, drop) {
  const taking = new $Steps(
    () => {
      if (taking.n === 0) return { done: true, value: undefined };
      taking.n--;
      return taking.iter.next();
    },
    () => $closed(taking.iter),
  );
  taking.iter = iter;
  taking.n = n;
  // Std's `Take::size_hint`, of what it has left to take.
  taking.sizeHint = () => $takeHint(taking.iter, taking.n);
  // Std's `Take::next_back`: its source's `nth_back` past what it won't
  // give, the crate's own where it reaches it (#220).
  taking.back = () => {
    if (taking.n === 0) return { done: true, value: undefined };
    const past = Math.max(taking.iter.len() - taking.n, 0);
    taking.n--;
    return $nthBackOf(taking.iter, past, drop);
  };
  taking.len = () => Math.min(taking.iter.len(), taking.n);
  // Std's `Take::advance_by(m)` and `count`: its source's `advance_by`, by
  // as many as it has left, which drops what it passes itself where it
  // can, a reversed `Vec`'s as one slice, front to back, else each by
  // `drop`; its `count` then closed, as `count(self)` drops it, though its
  // source's step throws, a panic (#826).
  const advanceBy = (m, dropItem = drop) => {
    const min = Math.min(taking.n, m);
    const advanced = min - $advanceByOf(taking.iter, min, dropItem);
    taking.n -= advanced;
    return m - advanced;
  };
  const count = (dropItem = drop) => {
    const k = taking.n;
    taking.n = 0;
    const counted = k === 0 ? 0 : $consumed(taking, () => k - $advanceByOf(taking.iter, k, dropItem));
    taking.return();
    return counted;
  };
  $forward(taking, iter, { advanceBy, count });
  // And its `nth_back`, `rfold` and `try_rfold`, std's, by its source's
  // `nth_back` past what it won't give, where that reaches the crate's own.
  if (iter.nthBack !== undefined || iter.rfold !== undefined || iter.tryRfold !== undefined) {
    // What its source's `nth_back` past what it won't give finds, which
    // std's `Take` lets go of, dropped by `drop` (#718): `false` where it
    // found none.
    const passed = (k) => {
      const step = $nthBackOf(taking.iter, k, drop);
      if (step.done) return false;
      drop?.(step.value);
      return true;
    };
    const nthBack = (k) => {
      const len = taking.iter.len();
      if (taking.n > k) {
        const past = Math.max(len - taking.n, 0) + k;
        taking.n -= k + 1;
        return $nthBackOf(taking.iter, past, drop);
      }
      if (len > 0) passed(len - 1);
      return { done: true, value: undefined };
    };
    // Its source trimmed to what it gives, `false` where that ran out.
    const trimmed = () => {
      const len = taking.iter.len();
      return !(len > taking.n && !passed(len - taking.n - 1));
    };
    // Consumed, its source done with after (#225).
    // What drops with `f`, `ends`, its source's `rfold`'s, first (#734).
    // Its trimming taken while its frame holds only `init` and `f`: where
    // that throws, a panic in a stage's closure, `ends`, then `init`, by
    // `unwind`, then what's left, as `rfold(self, init, f)` drops its
    // parameters as it unwinds; of its `try_rfold`, by `&mut`, not what's
    // left (#826).
    const kept = (init, unwind, ends) => $stepHolding(trimmed, init, unwind, ends);
    const rfold = (init, f, unwind, ends) => {
      const folded =
        taking.n === 0 || !$consumed(taking, () => kept(init, unwind, ends), ends)
          ? init
          : $foldBack(taking.iter, init, f, unwind, ends);
      $closedAfter(ends, () => $closed(taking.iter));
      return folded;
    };
    const tryRfold = (init, f, broke, unwind, ends) =>
      taking.n === 0 || !kept(init, unwind, ends) ? init : $tryRfoldOf(taking.iter, init, f, broke, unwind, ends);
    $forward(taking, iter, { nthBack, rfold, tryRfold });
  }
  return taking;
}
