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
      // And of one with a `try_fold` of its own, whose `nth` is std's
      // default, `advance_by` by it, as std's `Skip::next` asks (#294).
      if (skipping.n > 0 && (skipping.iter.ownNth || Object.hasOwn(skipping.iter, "tryFold"))) {
        const n = skipping.n;
        skipping.n = 0;
        return skipping.iter.nth(n, drop);
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
  // Each item's drop, which a `take` of it drops what it gives on by (#261).
  if (iter.ownNth) skipping.dropItem = drop;
  // Std's `Skip::advance_by(m)`: its source's `advance_by(n + m)`, which
  // drops what it passes itself where it can, a reversed `Vec`'s as one
  // slice, else each by the item's drop (#287).
  skipping.advanceBy = (m, dropItem = drop) => {
    const skip = skipping.n;
    const remainder = $advanceByOf(skipping.iter, skip + m, dropItem);
    const advanced = skip + m - remainder;
    skipping.n = Math.max(skip - advanced, 0);
    return m - Math.max(advanced - skip, 0);
  };
  // Std's `Skip::count`: its source's `nth(n - 1)`, then its `count`, where
  // its source drops what it passes over itself (#261).
  if (iter.ownNth) {
    skipping.count = (dropItem) => {
      if (skipping.n > 0) {
        const k = skipping.n;
        skipping.n = 0;
        const step = skipping.iter.nth(k - 1);
        if (step.done) {
          skipping.return();
          return 0;
        }
        dropItem?.(step.value);
      }
      return $count(skipping.iter, dropItem);
    };
  }
  // Std's `Skip::fold` and `try_fold`: its source's `nth(n - 1)`, where
  // that drops what it passes over itself, then its source's; its `fold`,
  // consuming it, closed at its end (#250).
  if (iter.ownNth) {
    skipping.fold = (init, f) => {
      if (skipping.n > 0) {
        const k = skipping.n;
        skipping.n = 0;
        const step = skipping.iter.nth(k - 1);
        if (step.done) {
          skipping.return();
          return init;
        }
        drop?.(step.value);
      }
      const folded = skipping.iter.fold(init, f);
      skipping.return();
      return folded;
    };
    // And its `try_fold`, the same first step, then its source's.
    skipping.tryFold = (init, f, broke) => {
      if (skipping.n > 0) {
        const k = skipping.n;
        skipping.n = 0;
        const step = skipping.iter.nth(k - 1);
        if (step.done) return init;
        drop?.(step.value);
      }
      return skipping.iter.tryFold(init, f, broke);
    };
  }
  // What std's `Skip::last` steps after its `nth(n - 1)?`: its source, what
  // it skips dropped, or `undefined` where that found none (#232).
  skipping.skipped = () => {
    if (skipping.n > 0 && skipping.iter.ownNth) {
      const k = skipping.n;
      skipping.n = 0;
      const step = skipping.iter.nth(k - 1);
      if (step.done) return undefined;
      drop?.(step.value);
      return skipping.iter;
    }
    for (; skipping.n > 0; skipping.n--) {
      const step = skipping.iter.next();
      if (step.done) {
        skipping.n = 0;
        return undefined;
      }
      drop?.(step.value);
    }
    return skipping.iter;
  };
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
