// `a.chain(b)`, lazy: each of `a`'s, then each of `b`'s; `a` closed once
// it's done, as std's lets it go, and `b` stepped on after a `None`, as
// std's is; each closed as it ends, which drops what each holds (ADR 0465).
// Its fold, std's, `a`'s then `b`'s, which an iterator rust-js makes routes
// through the crate's own (#162), and its `try_fold`, `a`'s then `b`'s.
function $lazyChain(a, b) {
  let first = a[Symbol.iterator]();
  const second = b[Symbol.iterator]();
  const chained = new $Steps(
    () => {
      if (first !== undefined) {
        const step = first.next();
        if (!step.done) return step;
        first.return?.();
        first = undefined;
      }
      const step = second.next();
      if (step.done) second.return?.();
      return step;
    },
    () => {
      first?.return?.();
      second.return?.();
    },
  );
  chained.fold = (init, f) => {
    let folded = init;
    if (first !== undefined) {
      folded = first.reduce(f, folded);
      first.return?.();
      first = undefined;
    }
    folded = second.reduce(f, folded);
    second.return?.();
    return folded;
  };
  // Std's `Chain::last`: `a`'s `last`, then `b`'s, both asked, `b`'s where
  // it has one.
  chained.last = () => {
    const last = (it) => (it.last !== undefined ? it.last() : $lent(it).last());
    const a = first !== undefined ? last(first) : { done: true, value: undefined };
    first = undefined;
    const b = last(second);
    return b.done ? a : b;
  };
  // Std's `Chain::nth`: `a` advanced, then its next, or what's left of
  // `n` of `b`'s own `nth`, where it has one (#214).
  chained.nth = (n) => {
    if (first !== undefined) {
      const left = first.advanceBy !== undefined ? first.advanceBy(n) : $Steps.prototype.advanceBy.call($lent(first), n);
      if (left === 0) {
        const step = first.next();
        if (!step.done) return step;
        n = 0;
      } else {
        n = left;
      }
      first.return?.();
      first = undefined;
    }
    return second.nth !== undefined ? second.nth(n) : $lent(second).nth(n);
  };
  chained.tryFold = (init, f, broke) => {
    let folded = init;
    if (first !== undefined) {
      folded = $lent(first).tryFold(folded, f, broke);
      if (broke(folded)) return folded;
      first.return?.();
      first = undefined;
    }
    folded = $lent(second).tryFold(folded, f, broke);
    if (!broke(folded)) second.return?.();
    return folded;
  };
  return chained;
}
