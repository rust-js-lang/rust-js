
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
function $iterator(iterator, next, boxed = false, nextBack, len, hint, fold, nth, last, rfold, nthBack, tryRfold) {
  // A range's, stepped by its own step, has std's `Range::nth` (#231), and
  // its `size_hint`, exact, of its bounds (ADR 0918).
  nth ??= $rangeNthOf(next);
  hint ??= $rangeHintOf(next);
  const step = (item) => (item == null ? { done: true, value: undefined } : { done: false, value: boxed ? $someValue(item) : item });
  const it = new $Steps(() => step(next(iterator)));
  // The crate's own `fold` and `nth`, where its impl writes them, which
  // what std routes through them calls (#162).
  // That `fold` takes what it steps and drops it, which `ownFold` says.
  if (fold !== undefined) {
    it.fold = (init, f) => fold(iterator, init, f);
    it.ownFold = true;
  }
  // `ownNth` and `ownLast` say they're the crate's own, which drop what
  // they pass themselves, where std's defaults of a `$Steps` don't.
  if (nth !== undefined) {
    it.nth = (k) => step(nth(iterator, k));
    it.ownNth = true;
  }
  // And its own `last`, which std's `last`, and a `skip`'s, a chain's, a
  // fuse's and a peekable's, call (#182).
  if (last !== undefined) {
    it.last = () => step(last(iterator));
    it.ownLast = true;
  }
  // That `rfold` takes what it steps and drops it, as its `fold` does.
  if (rfold !== undefined) {
    it.rfold = (init, f) => rfold(iterator, init, f);
    it.ownRfold = true;
  }
  if (nthBack !== undefined) it.nthBack = (k) => step(nthBack(iterator, k));
  if (tryRfold !== undefined) it.tryRfold = (init, f, broke) => tryRfold(iterator, init, f, broke);
  it.of = iterator;
  // Another of a value of its type, stepped as it is: a clone's (ADR 0473).
  it.with = (value) => $iterator(value, next, boxed, nextBack, len, hint, fold, nth, last, rfold, nthBack, tryRfold);
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
  // 0912): itself, whose own `last`, `nth`, `fold` and `advanceBy` are std's
  // for its kind, a chain's sides' (#276). Its reader by index, which steps
  // nothing, aside (ADR 0975).
  const own = (dictionary, steps) =>
    Object.entries(dictionary).every(([name, step]) => name === "getUnchecked" || steps[name] === step);
  if (own(iterator, { next: $next, fold: $foldOf, nth: $nth, last: $last }) && len === undefined
    && (back === undefined || own(back, { next_back: $nextBack }))) {
    return value;
  }
  return $iterator(
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
  );
}
