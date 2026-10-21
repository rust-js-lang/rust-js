// An iterator an adapter holds, closed when the adapter ends early, is
// dropped, or is done: `drop`, its own drop, given what it hasn't given,
// once, as Rust drops it with the adapter (ADRs 0422, 0465, 0570); its back
// still its own, where it has one (ADR 0514).
function $closing(it, drop) {
  let closed = false;
  const closing = Object.create(Iterator.prototype);
  // Stepped to its end, it's done with, as a chain consumed to its end
  // drops what it holds where the statement or the loop ends (ADR 0570).
  closing.next = () => {
    const step = it.next();
    if (step.done) closing.return();
    return step;
  };
  closing.back = () => it.back();
  closing.return = () => {
    if (!closed) {
      closed = true;
      drop?.(it);
    }
    return { done: true, value: undefined };
  };
  return closing;
}
