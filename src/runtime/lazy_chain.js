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
