// `iter::repeat_with(f)`: `f()` each step, forever. Its hint std's,
// `(usize::MAX, None)` (repeat_with.rs:83), which what holds it asks, an
// explicit `Take`'s.
function $repeatingWith(f) {
  const it = (function* () {
    while (true) {
      yield f();
    }
  })();
  it.sizeHint = () => [2 ** 32 - 1, undefined];
  return it;
}
