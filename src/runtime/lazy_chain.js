// `a.chain(b)`, lazy, Rust's `Chain`, the runtime's one (ADR 0545's #739
// amendment): each of `a`'s, then each of `b`'s; `a` closed once it's done
// from the front, as std's lets it go, and `b` stepped on after a `None`,
// as std's is; each closed as it ends, which drops what each holds (ADR
// 0465). Where both know their back, a chain the crate steps from its back
// makes them so (`$knowing`, ADR 0545), it knows its own, as std's
// `Chain` is double-ended where both are: `b`'s then `a`'s from the back,
// `b` let go once it's done from there. Its steps are std's, each side's,
// the crate's own where they reach it: from the front `fold`, `last`,
// `nth`, `try_fold`, `count` and `advance_by` (#162), from the back
// `rfold`, `try_rfold`, `nth_back` and `advance_back_by` (#220). Closing it
// closes `a`, then `b`, the second though the first's drop throws, as drop
// glue drops each field (`$closeEach`, #743).
function $lazyChain(a, b) {
  const done = { done: true, value: undefined };
  let first = $stepsOf(a);
  let second = $stepsOf(b);
  // The side that's done from where it's stepped let go, closed, as std's
  // `and_then_or_clear` drops it; the other stepped on after a `None`, as
  // std's is, closed there unless it's held, which its owner drops (#243).
  const chained = new $Steps(
    () => {
      if (first !== undefined) {
        const step = first.next();
        if (!step.done) return step;
        first.return?.();
        first = undefined;
      }
      if (second === undefined) return done;
      const step = second.next();
      if (step.done && !chained.held) second.return?.();
      return step;
    },
    () => $closeEach(first, second),
  );
  // Closed though a fold throws, a panic, `b` with what's left of `a`, as
  // std's `Chain::fold` drops itself as it unwinds (#423).
  // What drops with `f`, `ends`, given to `b`'s fold alone, as
  // `Chain::fold` gives `a`'s `&mut f` and `b`'s `f` (#734). Each side
  // closed once its fold is done with it, as `a.fold(acc, &mut f)` takes
  // `a`: never set aside unclosed, which a held side, `$consume`'s, needs
  // (#757).
  const fold = (init, f, unwind, ends) =>
    $consumed(
      chained,
      () => {
        let folded = init;
        if (first !== undefined) {
          folded = $foldDropping(first, folded, f, unwind);
          first.return?.();
          first = undefined;
        }
        if (second === undefined) {
          ends?.();
          return folded;
        }
        folded = $foldDropping(second, folded, f, unwind, ends);
        $closedAfter(ends, () => second.return?.());
        return folded;
      },
      ends,
    );
  // Std's `Chain::last`: `a`'s `last`, then `b`'s, both asked, `b`'s where
  // it has one; closed though `a`'s throws, a panic, `b` with it, as
  // `last(self)` drops what it holds as it unwinds (#647).
  const last = () =>
    $consumed(chained, () => {
      const lastOf = (it) =>
        it === undefined ? done : it.last !== undefined ? it.last() : $lent(it).last();
      const a = lastOf(first);
      first = undefined;
      const b = lastOf(second);
      return b.done ? a : b;
    });
  // Std's `Chain::nth`: `a` advanced, then its next, or what's left of
  // `n` of `b`'s own `nth`, where it has one (#214), what it passes over
  // dropped as `a`'s is (ADR 0925).
  const nth = (n, drop) => {
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
    if (second === undefined) return done;
    const dropping = second.dropItem ?? drop;
    return second.nth !== undefined ? second.nth(n, dropping) : $lent(second).nth(n, dropping);
  };
  // Std's `Chain::try_fold`: `a`'s by `&mut f`, let go once it's run out,
  // then `b`'s, given `f` and so what drops with it, `ends` (#768).
  const tryFold = (init, f, broke, unwind, ends) => {
    let folded = init;
    if (first !== undefined) {
      folded = $lent(first).tryFold(folded, f, broke, unwind);
      if (broke(folded)) return folded;
      first.return?.();
      first = undefined;
    }
    if (second === undefined) return folded;
    folded = $lent(second).tryFold(folded, f, broke, unwind, ends);
    if (!broke(folded) && !chained.held) second.return?.();
    return folded;
  };
  // Std's `Chain::count`: `a`'s own `count`, then `b`'s, a `Take`'s
  // advancing its source, which drops what it passes as one slice (#283);
  // closed though `a`'s throws, a panic, `b` with it, as `count(self)`
  // drops what it holds as it unwinds (#647).
  const count = (drop) =>
    $consumed(chained, () => {
      let counted = 0;
      if (first !== undefined) {
        counted = $count(first, drop);
        first = undefined;
      }
      return second === undefined ? counted : counted + $count(second, drop);
    });
  // Std's `Chain::advance_by`: `a`'s, then `b`'s of what's left, `a` let
  // go once it's run out (#250), each given the items' drop its caller
  // gives (ADR 0925).
  const advanced = (it, m, drop) =>
    it.advanceBy !== undefined ? it.advanceBy(m, drop) : $Steps.prototype.advanceBy.call($lent(it), m, drop);
  const advanceBy = (m, drop) => {
    if (first !== undefined) {
      m = advanced(first, m, drop);
      if (m === 0) return 0;
      first.return?.();
      first = undefined;
    }
    return second === undefined ? m : advanced(second, m, drop);
  };
  $forward(chained, first, { fold, last, nth, tryFold, count, advanceBy });
  if (first.back !== undefined && second.back !== undefined) {
    chained.back = () => {
      if (second !== undefined) {
        const step = second.back();
        if (!step.done) return step;
        second.return?.();
        second = undefined;
      }
      if (first === undefined) return done;
      const step = first.back();
      if (step.done && !chained.held) first.return?.();
      return step;
    };
    // Std's `Chain::rfold`: `b`'s, then `a`'s, closed though `b`'s throws,
    // `a` with it, as `rfold(self)` drops itself as it unwinds; what drops
    // with `f`, `ends`, given to `a`'s alone, as `Chain::rfold` gives `b`'s
    // `&mut f` and `a`'s `f` (#734).
    const rfold = (init, f, unwind, ends) =>
      $consumed(
        chained,
        () => {
          let folded = init;
          if (second !== undefined) {
            folded = $foldBack(second, folded, f, unwind);
            second.return?.();
            second = undefined;
          }
          if (first === undefined) {
            ends?.();
            return folded;
          }
          folded = $foldBack(first, folded, f, unwind, ends);
          $closedAfter(ends, () => first.return?.());
          first = undefined;
          return folded;
        },
        ends,
      );
    // Std's `Chain::try_rfold`: `b`'s, let go once it's run out, then `a`'s,
    // given `f` and so what drops with it, `ends` (#768).
    const tryRfold = (init, f, broke, unwind, ends) => {
      let folded = init;
      if (second !== undefined) {
        folded = $tryRfoldOf(second, folded, f, broke, unwind);
        if (broke(folded)) return folded;
        second.return?.();
        second = undefined;
      }
      return first === undefined ? folded : $tryRfoldOf(first, folded, f, broke, unwind, ends);
    };
    // Std's `Chain::advance_back_by`: `b`'s, then `a`'s of what's left,
    // `b` let go once it's run out; each its own, a `Vec`'s as one slice,
    // else each `next_back` dropped by `drop` (#752).
    const backBy = (it, m, drop) => {
      if (it.advanceBackBy !== undefined) return it.advanceBackBy(m, it.dropItem ?? drop);
      for (; m > 0; m--) {
        const step = it.back();
        if (step.done) break;
        drop?.(step.value);
      }
      return m;
    };
    const advanceBackBy = (m, drop) => {
      if (second !== undefined) {
        m = backBy(second, m, drop);
        if (m === 0) return 0;
        second.return?.();
        second = undefined;
      }
      return first === undefined ? m : backBy(first, m, drop);
    };
    // Std's `Chain::nth_back`: `b` advanced back past `n`, which drops what
    // it passes, then its `next_back`; or what's left of `n` of `a`'s own
    // `nth_back` (#752).
    const nthBack = (n, drop) => {
      if (second !== undefined) {
        n = backBy(second, n, drop);
        if (n === 0) {
          const step = second.back();
          if (!step.done) return step;
        }
        second.return?.();
        second = undefined;
      }
      if (first === undefined) return done;
      if (first.nthBack !== undefined) return first.nthBack(n, first.dropItem ?? drop);
      return $nthBackOf(first, n, first.dropItem ?? drop);
    };
    $forward(chained, first, { rfold, tryRfold, advanceBackBy, nthBack });
  }
  // Its `nth` drops what it passes over itself where each side's does, a
  // `Vec`'s (#250).
  // Its `count`, each side's, the crate's own where one writes it (#505).
  if (first.ownCount || second.ownCount) $owns(chained, "count");
  if (first.ownNth && second.ownNth) {
    $owns(chained, "nth");
    chained.dropItem = first.dropItem ?? second.dropItem;
  }
  // Its sides, `a` let go as it's taken, as std's `last` takes them
  // (`$lastDropping`, #232).
  // Std's `Chain::size_hint` (chain.rs:182): of the side it has left, or
  // of both, summed, its first side asked first; of none, none.
  chained.sizeHint = () => {
    if (first === undefined) return second === undefined ? [0, 0] : $sizeHint(second);
    return second === undefined ? $sizeHint(first) : $hintSum($sizeHint(first), $sizeHint(second));
  };
  chained.sides = () => {
    const a = first;
    first = undefined;
    return [a, second];
  };
  chained.from = [first, second];
  return chained;
}
