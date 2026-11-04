
// A Rust iterator of the crate's own as a JS one: its `next` returns the
// item, or `None` at the end. JS's iterator helpers are lazy, like Rust's
// adapters, so an endless one is fine until something wants all of it. A
// generic `next` may box its `Some` (ADR 0051): `boxed` unboxes it. `of`
// is what it's of, which its drop drops (ADR 0421). Given its `nextBack`,
// it knows its back, which `$nextBack` steps; given its `len`, its length,
// an `ExactSizeIterator`'s (ADR 0456). Given its own `fold` and `nth`, the
// crate's impl's, std's consumers call them (#162); and its own `rfold` and
// `nth_back`, which std's consumers and adapters stepped from the back call
// (#220), and `try_rfold`, std's compiled struct's, a `Take`'s, which what
// folds it from its back asks (ADR 0930).
function $iterator(iterator, next, boxed = false, nextBack, len, hint, fold, nth, last, rfold, nthBack, tryRfold, count) {
  // A range's, stepped by its own step, has std's `Range::nth` (#231), and
  // its `size_hint`, exact, of its bounds (ADR 0918).
  nth ??= $rangeNthOf(next);
  hint ??= $rangeHintOf(next);
  const step = (item) => (item == null ? { done: true, value: undefined } : { done: false, value: boxed ? $someValue(item) : item });
  const it = new $Steps(() => step(next(iterator)));
  // An inclusive range's own `try_fold`, std's, which what folds it asks,
  // and so its `fold`: they end it as std's do, exhausted at its last (#414).
  // And from its back, by its `next_back`: its `try_rfold`, and so its
  // `rfold`, std's, which end it exhausted at its first, as a held `Rev`'s
  // fold of it does, not past it as `next_back` does (#414).
  const rangeFold = $rangeFoldOf(next);
  const rangeRfold = nextBack === undefined ? undefined : $rangeFoldOf(nextBack);
  // The crate's own `fold` and `nth`, where its impl writes them, which
  // what std routes through them calls (#162), before a range's.
  // That `fold` takes what it steps and drops it, which `ownFold` says.
  // Given `unwind`, the drop of an accumulator a stage's panic may strand,
  // `$foldDropping`'s: each dictionary's `fold` and `rfold` takes it after
  // its function, as its `B`'s drop, the crate's own's first drop and std's
  // compiled struct's (#423). Its own `last`, which std's `last`, and a
  // `skip`'s, a chain's, a fuse's and a peekable's, call (#182); its own
  // `rfold`, which takes what it steps and drops it, as its `fold` does; and
  // its own `count`, which takes it and drops it itself, as std's adapters
  // whose `count` is their source's, and generic code's, call it (#505).
  $forward(it, iterator, {
    tryFold: rangeFold && ((init, f, broke) => rangeFold(iterator, init, f, broke)),
    fold:
      fold === undefined
        ? rangeFold && ((init, f) => rangeFold(iterator, init, f, () => false))
        : (init, f, unwind) => fold(iterator, init, f, unwind),
    tryRfold:
      tryRfold === undefined
        ? rangeRfold && ((init, f, broke) => rangeRfold(iterator, init, f, broke))
        : (init, f, broke, unwind) => tryRfold(iterator, init, f, broke, unwind),
    rfold:
      rfold === undefined
        ? rangeRfold && ((init, f) => rangeRfold(iterator, init, f, () => false))
        : (init, f, unwind) => rfold(iterator, init, f, unwind),
    nth: nth && ((k) => step(nth(iterator, k))),
    last: last && (() => step(last(iterator))),
    nthBack: nthBack && ((k) => step(nthBack(iterator, k))),
    count: count && (() => count(iterator)),
  });
  // `ownNth` and `ownLast` say they're the crate's own, which drop what
  // they pass themselves, where std's defaults of a `$Steps` don't.
  if (fold !== undefined) it.ownFold = true;
  if (nth !== undefined) it.ownNth = true;
  if (last !== undefined) it.ownLast = true;
  if (rfold !== undefined) it.ownRfold = true;
  if (count !== undefined) it.ownCount = true;
  it.of = iterator;
  // Another of a value of its type, stepped as it is: a clone's (ADR 0525,
  // amended).
  it.with = (value) => $iterator(value, next, boxed, nextBack, len, hint, fold, nth, last, rfold, nthBack, tryRfold, count);
  if (nextBack !== undefined) {
    it.back = () => step(nextBack(iterator));
  }
  if (len !== undefined) {
    it.len = () => len(iterator);
  }
  // Its own `size_hint`, the crate's impl's, which `$sizeHint` reads.
  if (hint !== undefined) {
    it.hint = () => hint(iterator);
  }
  return it;
}

// `it`, a compiled std iterator's struct (ADR 0561), `$stdIterator`'s,
// given its back and length, a use site's, and its compiled `rfold` and
// `nth_back`, which pass the crate's own on (#224).
function $knowing(it, back, len, rfold, nthBack, tryRfold) {
  let source = it;
  while (source.knowing === undefined && source.iter !== undefined) source = source.iter;
  source.knowing?.(back, len, rfold, nthBack, tryRfold);
  return it;
}

// A value of a type parameter given as it is (ADR 0411), as a JS iterator
// stepped by its `Iterator` dictionary, `iterator`: its `next`, and its own
// `fold`, `nth` and `size_hint` where the crate's impl writes them (#162);
// `boxed` and its length as `$iterator`'s; its back by its
// `DoubleEndedIterator` dictionary, `back`: its `next_back`, and its own
// `rfold` and `nth_back` where the crate's impl writes them (#224).
function $given(value, iterator, boxed = false, back, len) {
  // A JS iterator stepped as it is, its dictionary its own steps, each one
  // that calls the value's, `$next` and `$foldOf` of std's struct (ADR
  // 0912): itself, whose own `last`, `nth`, `fold`, `count` and `advanceBy`
  // are std's for its kind, a chain's sides' (#276, #505). Its reader by
  // index, which steps nothing, aside (ADR 0975). Made a JS iterator again,
  // it would lose its own `tryFold`, a `skip`'s source's `nth(n - 1)` (#520).
  // Given `boxed`, a step that boxes its `Some`, `$nextSome`'s, is the
  // value's too, which the JS iterator would unbox: generic code's of items
  // that may look like `None`, whose value may be JS's own, an array's, with
  // no `nth` to ask (#522).
  const own = (dictionary) =>
    Object.entries(dictionary).every(([name, step]) => name === "getUnchecked" || $askedStep(step, boxed) === name);
  if (own(iterator) && len === undefined && (back === undefined || own(back))) {
    return value;
  }
  const it = $iterator(
    value,
    iterator.next,
    boxed,
    back?.next_back,
    len,
    iterator.sizeHint,
    iterator.fold,
    iterator.nth,
    iterator.last,
    back?.rfold,
    back?.nthBack,
    back?.tryRfold,
    iterator.count,
  );
  // Made of the value, which `$givenClone` clones.
  it.given = true;
  return it;
}

// Which step `step`, a dictionary's, is, where it's one of std's helpers
// that asks the value itself, `$foldOf`'s `fold`, its name in the
// dictionary: the one table of them (ADR 1089). A dictionary of only these
// is the value's own steps, which `$given` gives as it is; of one of them,
// whether it took the value is the value's own answer, `$took`'s. Given
// `boxed`, a step that boxes its `Some`, `$nextSome`'s, too (#522).
function $askedStep(step, boxed = false) {
  if (step === $next || (boxed && step === $nextSome)) return "next";
  if (step === $nextBack || (boxed && step === $nextBackSome)) return "next_back";
  if (step === $foldOf) return "fold";
  if (step === $foldBack) return "rfold";
  if (step === $nth || (boxed && step === $nthSome)) return "nth";
  if (step === $last || (boxed && step === $lastSome)) return "last";
  if (step === $count) return "count";
  return undefined;
}

// Whether a dictionary's `step` of `it`, a value of a type parameter, took
// it whole and dropped it, as the crate's own `fold`, `last` and `count` do,
// so generic code drops nothing of it after: none didn't; the crate's own,
// or std's compiled one, did; std's helper that asks the value, `$foldOf`'s,
// did where the value's own step ran, the crate's (`ownFold`) or std's
// compiled struct's, which it takes (`takes`). A dictionary can't say which
// of a type parameter's values is which: generic code's of a `fuse()` of one
// it was given asks it at run time (#621, #649).
function $took(step, it) {
  if (step === undefined) return false;
  const name = $askedStep(step, true);
  return name === undefined || it[$ownStep(name)] === true || it.$steps?.takes?.includes(name) === true;
}

// A clone of `it`, what generic code's `copied()` of a value it was given
// is, `$given`'s, by `clone`, the value's `Clone` (#384): where `$given`
// made a JS iterator of the value, another of the value's clone, stepped
// as it is; else the value itself, `clone`'s.
function $givenClone(it, clone) {
  if (it.given !== true) return clone(it);
  const again = it.with(clone(it.of));
  again.given = true;
  return again;
}
