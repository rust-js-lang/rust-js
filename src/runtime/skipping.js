// `it.skip(n)` explicit, Rust's `Skip` (ADR 0542): what it steps, `iter`,
// and how many it has left to skip, `n`, kept, which a clone reads. The
// first `n` are skipped as it's first stepped, each dropped by `drop`
// where items have a destructor, as Rust's `nth` drops what it skips;
// closing it closes what it steps, which drops what it holds (ADRs 0422,
// 0465). Its back and length are its source's less what it skips, as
// Rust's `next_back` and `len` are.
function $skipping(iter, n, drop) {
  const skipping = new $Steps(
    () => {
      // Std's `Skip::next`, its source's `nth(n)`, where that drops what
      // it passes over itself, a reversed `Vec`'s as one slice (#234).
      if (skipping.n > 0 && skipping.iter.ownNth) {
        const n = skipping.n;
        skipping.n = 0;
        return skipping.iter.nth(n);
      }
      for (; skipping.n > 0; skipping.n--) {
        const step = skipping.iter.next();
        if (step.done) {
          skipping.n = 0;
          return step;
        }
        drop?.(step.value);
      }
      return skipping.iter.next();
    },
    () => $closed(skipping.iter),
  );
  skipping.iter = iter;
  skipping.n = n;
  // Std's `Skip::nth(k)`, its source's `nth(n + k)`, where that drops what
  // it passes over itself, a reversed `Vec`'s as one slice (#234).
  if (iter.ownNth) {
    skipping.nth = (k) => {
      const skip = skipping.n;
      skipping.n = 0;
      return skipping.iter.nth(skip + k);
    };
    skipping.ownNth = true;
  }
  skipping.hint = () => {
    const [low, high] = $sizeHint(skipping.iter);
    return [Math.max(low - skipping.n, 0), high === undefined ? undefined : Math.max(high - skipping.n, 0)];
  };
  skipping.back = () => (skipping.len() > 0 ? skipping.iter.back() : { done: true, value: undefined });
  skipping.len = () => Math.max(skipping.iter.len() - skipping.n, 0);
  // Std's `Skip::nth_back`: its source's, of what it gives, the crate's own
  // where it reaches it (#220).
  if (iter.nthBack !== undefined) {
    skipping.nthBack = (k) => {
      const len = skipping.len();
      if (k < len) return skipping.iter.nthBack(k);
      if (len > 0) skipping.iter.nthBack(len - 1);
      return { done: true, value: undefined };
    };
  }
  // Its `try_rfold` and `rfold`, std's: its source's, stopped after what it
  // gives.
  if (iter.tryRfold !== undefined) {
    skipping.tryRfold = (init, f, broke) => {
      let left = skipping.len();
      if (left === 0) return init;
      return skipping.iter.tryRfold(init, (a, x) => (left--, f(a, x)), (a) => left === 0 || broke(a));
    };
    // Consumed, its source done with after (#225).
    skipping.rfold = (init, f) => {
      const folded = skipping.tryRfold(init, f, () => false);
      $closed(skipping.iter);
      return folded;
    };
  }
  return skipping;
}
