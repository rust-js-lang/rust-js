// An iterator an adapter holds, closed when the adapter ends early or is
// dropped: `drop`, its own drop, given what it hasn't given, once, as Rust
// drops it with the adapter (ADRs 0422, 0465).
function $closing(it, drop) {
  let closed = false;
  return Iterator.from({
    next() {
      return it.next();
    },
    return() {
      if (!closed) {
        closed = true;
        drop?.(it);
      }
      return { done: true, value: undefined };
    },
  });
}
