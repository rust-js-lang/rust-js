// An iterator an adapter holds, closed when the adapter ends early, is
// dropped, or is done, however it's stepped: `drop`, its own drop, given what it hasn't given,
// once, as Rust drops it with the adapter (ADRs 0422, 0465, 0570); its back
// still its own, where it has one (ADR 0514). It's a `$Steps` (ADR 0669):
// its fold is its iterator's, after which it's done with; the crate's own
// fold takes it and drops it itself (#162).
function $closing(it, drop) {
  let closed = false;
  const close = () => {
    if (!closed) {
      closed = true;
      drop?.(it);
    }
  };
  // Stepped to its end, it's done with, as a chain consumed to its end
  // drops what it holds where the statement or the loop ends (ADR 0570).
  // Done, from its front, its back or by its `nth` or `nth_back`, it's
  // done with, each way it's stepped (#225).
  const done = (step) => {
    if (step.done) close();
    return step;
  };
  const closing = new $Steps(() => done(it.next()), close);
  closing.back = () => done(it.back());
  // Its length, its iterator's, where it knows it: what an `enumerate`
  // stepped from its back counts by (#211).
  if (it.len !== undefined) closing.len = () => it.len();
  // Its iterator's back and length, a use site's, given it where it's
  // asked, as `$knowing` gives a compiled std iterator's (ADR 0561, #225).
  if (it.knowing !== undefined) {
    closing.knowing = (back, size) => {
      it.knowing(back, size);
      if (it.len !== undefined) closing.len = () => it.len();
    };
  }
  // Its `nth`, its iterator's, the crate's own where it writes one.
  if (it.nth !== undefined) closing.nth = (k) => done(it.nth(k));
  closing.fold = (init, f) => {
    const folded = it.fold !== undefined ? it.fold(init, f) : it.reduce((a, x) => f(a, x), init);
    if (it.ownFold) closed = true;
    close();
    return folded;
  };
  // Its `nth_back` and `rfold`, its iterator's, the crate's own where it
  // writes them, done with after its `rfold` as after its `fold` (#220).
  if (it.nthBack !== undefined) closing.nthBack = (k) => done(it.nthBack(k));
  if (it.rfold !== undefined) {
    closing.rfold = (init, f) => {
      const folded = it.rfold(init, f);
      if (it.ownRfold) closed = true;
      close();
      return folded;
    };
  }
  if (it.tryRfold !== undefined) closing.tryRfold = (init, f, broke) => it.tryRfold(init, f, broke);
  // Its `try_fold`, its iterator's, done with where it isn't stopped.
  closing.tryFold = (init, f, broke) => {
    const folded = $lent(it).tryFold(init, f, broke);
    if (!broke(folded)) close();
    return folded;
  };
  return closing;
}
