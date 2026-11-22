// `iter::once_with(f)`: `f` called when it is first stepped, its value the
// one item. Its hint std's, `(1, Some(1))` while it holds `f`, else `(0,
// Some(0))` (sources/once_with.rs), which what holds it asks, a
// `peekable()`'s or a `Vec` grown by it (#1375).
function $onceWith(f) {
  let held = true;
  const it = Object.create(Iterator.prototype);
  it.next = () => {
    if (!held) return { done: true, value: undefined };
    held = false;
    return { done: false, value: f() };
  };
  it.sizeHint = () => (held ? [1, 1] : [0, 0]);
  it.return = () => {
    held = false;
    return { done: true, value: undefined };
  };
  return it;
}
