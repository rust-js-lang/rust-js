// An iterator an adapter holds, closed when the adapter ends early or is
// dropped: `drop`, its own drop, given what it hasn't given, once, as Rust
// drops it with the adapter (ADRs 0422, 0465); its back and its length
// still its own, where it has them (ADR 0514).
function $closing(it, drop) {
  let closed = false;
  const closing = Object.create(Iterator.prototype);
  closing.next = () => it.next();
  closing.back = () => it.back();
  closing.len = () => it.len();
  closing.return = () => {
    if (!closed) {
      closed = true;
      drop?.(it);
    }
    return { done: true, value: undefined };
  };
  return closing;
}
