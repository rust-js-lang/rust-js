// An iterator an adapter holds, closed when the adapter ends early, is
// dropped, or is done: `drop`, its own drop, given what it hasn't given,
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
  const closing = new $Steps(() => {
    const step = it.next();
    if (step.done) close();
    return step;
  }, close);
  closing.back = () => it.back();
  closing.fold = (init, f) => {
    const folded = it.fold !== undefined ? it.fold(init, f) : it.reduce((a, x) => f(a, x), init);
    if (it.ownFold) closed = true;
    close();
    return folded;
  };
  return closing;
}
