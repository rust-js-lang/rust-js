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
  taking.hint = () => {
    const [low, high] = $sizeHint(taking.iter);
    return [Math.min(low, taking.n), high === undefined ? taking.n : Math.min(high, taking.n)];
  };
  // Std's `Take::next_back`: its source's `nth_back` past what it won't
  // give, the crate's own where it reaches it (#220).
  taking.back = () => {
    if (taking.n === 0) return { done: true, value: undefined };
    const past = Math.max(taking.iter.len() - taking.n, 0);
    taking.n--;
    return $nthBackOf(taking.iter, past, drop);
  };
  taking.len = () => Math.min(taking.iter.len(), taking.n);
  // And its `nth_back`, `rfold` and `try_rfold`, std's, by its source's
  // `nth_back` past what it won't give, where that reaches the crate's own.
  if (iter.nthBack !== undefined || iter.rfold !== undefined || iter.tryRfold !== undefined) {
    taking.nthBack = (k) => {
      const len = taking.iter.len();
      if (taking.n > k) {
        const past = Math.max(len - taking.n, 0) + k;
        taking.n -= k + 1;
        return $nthBackOf(taking.iter, past, drop);
      }
      if (len > 0) $nthBackOf(taking.iter, len - 1, drop);
      return { done: true, value: undefined };
    };
    // Its source trimmed to what it gives, `false` where that ran out.
    const trimmed = () => {
      const len = taking.iter.len();
      return !(len > taking.n && $nthBackOf(taking.iter, len - taking.n - 1, drop).done);
    };
    taking.rfold = (init, f) => (taking.n === 0 || !trimmed() ? init : $foldBack(taking.iter, init, f));
    taking.tryRfold = (init, f, broke) =>
      taking.n === 0 || !trimmed() ? init : $tryRfoldOf(taking.iter, init, f, broke);
  }
  return taking;
}
