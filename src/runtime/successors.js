// `iter::successors(first, f)`: each item, the next one `f` makes of it
// while that's `Some`. It holds the next one it gives: where it's closed
// before, early or never stepped, that one is dropped by `drop`, its
// item's, as Rust's `Successors` drops what it holds (ADR 0465). Its hint
// std's, `(1, None)` while it holds one, else `(0, Some(0))`
// (successors.rs:49), which what holds it asks, an explicit `Take`'s.
function $successors(first, successor, boxed = false, drop) {
  let next = first;
  const it = Object.create(Iterator.prototype);
  it.next = () => {
    if (next == null) return { done: true, value: undefined };
    const item = boxed ? $someValue(next) : next;
    next = successor(item);
    return { done: false, value: item };
  };
  it.sizeHint = () => (next == null ? [0, 0] : [1, undefined]);
  it.return = () => {
    if (next != null) {
      const held = boxed ? $someValue(next) : next;
      next = undefined;
      drop?.(held);
    }
    return { done: true, value: undefined };
  };
  return it;
}
