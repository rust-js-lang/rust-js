
// A Rust iterator of the crate's own as a JS one: its `next` returns the
// item, or `None` at the end. JS's iterator helpers are lazy, like Rust's
// adapters, so an endless one is fine until something wants all of it. A
// generic `next` may box its `Some` (ADR 0051): `boxed` unboxes it. `of`
// is what it's of, which its drop drops (ADR 0421). Given its `nextBack`,
// it knows its back, which `$nextBack` steps; given its `len`, its length,
// an `ExactSizeIterator`'s (ADR 0456). Given its own `fold` and `nth`, the
// crate's impl's, std's consumers call them (#162); and its own `rfold` and
// `nth_back`, which std's consumers and adapters stepped from the back call
// (#220).
function $iterator(iterator, next, boxed = false, nextBack, len, hint, fold, nth, last, rfold, nthBack) {
  // A range's, stepped by its own step, has std's `Range::nth` (#231).
  nth ??= $rangeNthOf(next);
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
  it.of = iterator;
  // Another of a value of its type, stepped as it is: a clone's (ADR 0473).
  it.with = (value) => $iterator(value, next, boxed, nextBack, len, hint, fold, nth, last, rfold, nthBack);
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
  // Its back and its length, a use site's, where they weren't given where
  // it was made, of the value it steps (ADR 0542).
  it.knowing = (back, size) => {
    if (back !== undefined) it.back = () => step(back(iterator));
    if (size !== undefined) it.len = () => size(iterator);
  };
  return it;
}

// `it`, an explicit adapter (ADR 0542), stepped by a use site that knows its
// source's back and length, `back` and `len`, functions of the value the
// `$iterator` it steps steps, which that's given: what it steps from its
// back as Rust's does. Or a compiled std iterator's struct (ADR 0561),
// `$stdIterator`'s, given its own, and its compiled `rfold` and `nth_back`,
// which pass the crate's own on (#224).
function $knowing(it, back, len, rfold, nthBack) {
  let source = it;
  while (source.knowing === undefined && source.iter !== undefined) source = source.iter;
  source.knowing?.(back, len, rfold, nthBack);
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
  // for its kind, a chain's sides' (#276).
  const own = (dictionary, steps) => Object.entries(dictionary).every(([name, step]) => steps[name] === step);
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
  );
}
