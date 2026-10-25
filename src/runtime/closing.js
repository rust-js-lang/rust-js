// An iterator an adapter holds, closed when the adapter ends early, is
// dropped, or is done, however it's stepped: `drop`, its own drop, given what it hasn't given,
// once, as Rust drops it with the adapter (ADRs 0422, 0465, 0570); its back
// still its own, where it has one (ADR 0514). It's a `$Steps` (ADR 0669):
// its fold is its iterator's, after which it's done with; the crate's own
// fold takes it and drops it itself (#162).
function $closing(it, drop, slices = true) {
  let closed = false;
  const close = () => {
    if (!closed) {
      closed = true;
      drop?.(it);
    }
  };
  // Stepped to its end, it's done with, as a chain consumed to its end
  // drops what it holds where the statement or the loop ends (ADR 0570).
  // Done, from its front or by its `nth` or `nth_back`, it's done with,
  // each way a consumer of JS's own steps it (#225).
  // Held, its owner drops it, where Rust does (#206).
  const done = (step) => {
    if (step.done && !closing.held) close();
    return step;
  };
  const closing = new $Steps(() => done(it.next()), close);
  closing.from = [it];
  // Of an array's items it owns, a `Vec`'s `IntoIter`'s, std's `nth_back`:
  // what it passes over dropped as one slice, front to back, as its
  // `advance_back_by` drops them, then its next from the back; not of one
  // that drops each as it's passed, `slices` false, a `Drain`'s (#234).
  if (slices && it.items !== undefined && it.nthBack === undefined && drop?.each !== undefined) {
    closing.advanceBackBy = (m) => {
      const n = Math.min(m, it.len());
      const passed = it.items.slice(it.end - n, it.end);
      it.end -= n;
      for (const item of passed) drop.each(item);
      return m - n;
    };
    closing.nthBack = (k) => done(closing.advanceBackBy(k) > 0 ? { done: true, value: undefined } : it.back());
    closing.ownNthBack = true;
    // And from its front, its `advance_by` and `nth`, what they pass over
    // dropped as one slice too, by itself, as a `chain`'s `nth` asks it
    // (#250); how many there weren't.
    closing.advanceBy = (m) => {
      const n = Math.min(m, it.len());
      const passed = it.items.slice(it.at, it.at + n);
      it.at += n;
      for (const item of passed) drop.each(item);
      return m - n;
    };
    closing.nth = (k) => done(closing.advanceBy(k) > 0 ? { done: true, value: undefined } : it.next());
    closing.ownNth = true;
  }
  closing.back = () => it.back();
  // Its length, its iterator's, where it knows it: what an `enumerate`
  // stepped from its back counts by (#211).
  if (it.len !== undefined) closing.len = () => it.len();
  // Its iterator's back and length, a use site's, given it where it's
  // asked, as `$knowing` gives a compiled std iterator's (ADR 0561, #225).
  if (it.knowing !== undefined) {
    closing.knowing = (back, size, rfold, nthBack) => {
      it.knowing(back, size, rfold, nthBack);
      if (it.len !== undefined) closing.len = () => it.len();
      if (it.rfold !== undefined) closing.rfold = rfoldClosing;
      if (it.nthBack !== undefined) closing.nthBack = (k) => done(it.nthBack(k));
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
  const rfoldClosing = (init, f) => {
    const folded = it.rfold(init, f);
    if (it.ownRfold) closed = true;
    close();
    return folded;
  };
  if (it.rfold !== undefined) closing.rfold = rfoldClosing;
  if (it.tryRfold !== undefined) closing.tryRfold = (init, f, broke) => it.tryRfold(init, f, broke);
  // Its `try_fold`, its iterator's, done with where it isn't stopped.
  closing.tryFold = (init, f, broke) => {
    const folded = $lent(it).tryFold(init, f, broke);
    if (!broke(folded) && !closing.held) close();
    return folded;
  };
  return closing;
}
