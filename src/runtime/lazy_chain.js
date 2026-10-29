// `a.chain(b)`, lazy: each of `a`'s, then each of `b`'s; `a` closed once
// it's done, as std's lets it go, and `b` stepped on after a `None`, as
// std's is; each closed as it ends, which drops what each holds (ADR 0465).
// Its fold, std's, `a`'s then `b`'s, which an iterator rust-js makes routes
// through the crate's own (#162), and its `try_fold`, `a`'s then `b`'s.
function $lazyChain(a, b) {
  let first = $stepsOf(a);
  const second = $stepsOf(b);
  const chained = new $Steps(
    () => {
      if (first !== undefined) {
        const step = first.next();
        if (!step.done) return step;
        first.return?.();
        first = undefined;
      }
      const step = second.next();
      // `b` done, never let go by std's, which its owner drops, held (#243).
      if (step.done && !chained.held) second.return?.();
      return step;
    },
    () => {
      first?.return?.();
      second.return?.();
    },
  );
  // Closed though a fold throws, a panic, `b` with what's left of `a`, as
  // std's `Chain::fold` drops itself as it unwinds (#423).
  chained.fold = (init, f, unwind) =>
    $consumed(chained, () => {
      let folded = init;
      if (first !== undefined) {
        folded = $foldDropping(first, folded, f, unwind);
        first.return?.();
        first = undefined;
      }
      folded = $foldDropping(second, folded, f, unwind);
      second.return?.();
      return folded;
    });
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
  // `n` of `b`'s own `nth`, where it has one (#214), what it passes over
  // dropped as `a`'s is (ADR 0925).
  chained.nth = (n, drop) => {
    if (first !== undefined) {
      // What it passes over dropped, by its own `advance_by` or each by
      // its item's drop (#269), the one given where it has none (#285).
      const left = advanced(first, n, first.dropItem ?? drop);
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
    const dropping = second.dropItem ?? drop;
    return second.nth !== undefined ? second.nth(n, dropping) : $lent(second).nth(n, dropping);
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
    if (!broke(folded) && !chained.held) second.return?.();
    return folded;
  };
  // Std's `Chain::count`: `a`'s own `count`, then `b`'s, a `Take`'s
  // advancing its source, which drops what it passes as one slice (#283).
  chained.count = (drop) => {
    let counted = 0;
    if (first !== undefined) {
      counted = $count(first, drop);
      first = undefined;
    }
    return counted + $count(second, drop);
  };
  // Std's `Chain::advance_by`: `a`'s, then `b`'s of what's left, `a` let
  // go once it's run out (#250), each given the items' drop its caller
  // gives (ADR 0925).
  const advanced = (it, m, drop) =>
    it.advanceBy !== undefined ? it.advanceBy(m, drop) : $Steps.prototype.advanceBy.call($lent(it), m, drop);
  chained.advanceBy = (m, drop) => {
    if (first !== undefined) {
      m = advanced(first, m, drop);
      if (m === 0) return 0;
      first.return?.();
      first = undefined;
    }
    return advanced(second, m, drop);
  };
  // Its `nth` drops what it passes over itself where each side's does, a
  // `Vec`'s (#250).
  if (first.ownNth && second.ownNth) {
    chained.ownNth = true;
    chained.dropItem = first.dropItem ?? second.dropItem;
  }
  // Its sides, `a` let go as it's taken, as std's `last` takes them
  // (`$lastDropping`, #232).
  chained.sides = () => {
    const a = first;
    first = undefined;
    return [a, second];
  };
  chained.from = [first, second];
  return chained;
}
