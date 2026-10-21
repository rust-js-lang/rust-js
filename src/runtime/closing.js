// An iterator an adapter holds, closed when the adapter ends early or is
// dropped: `drop`, its own drop, given what it hasn't given, once, as Rust
// drops it with the adapter (ADRs 0422, 0465); its back and its length
// still its own, where it has them (ADR 0514), or is given them where it's
// stepped, `iter`, what it closes (ADR 0549).
function $closing(it, drop) {
  let closed = false;
  const closing = Object.create(Iterator.prototype);
  closing.iter = it;
  // Stepped to its end, it's done with, as a chain consumed to its end
  // drops what it holds where the statement or the loop ends (ADR 0570).
  closing.next = () => {
    const step = it.next();
    if (step.done) closing.return();
    return step;
  };
  closing.back = () => it.back();
  closing.len = () => it.len();
  // Its size hint, what it closes' (ADR 0543).
  closing.hint = () => $sizeHint(it);
  closing.return = () => {
    if (!closed) {
      closed = true;
      drop?.(it);
    }
    return { done: true, value: undefined };
  };
  return closing;
}
