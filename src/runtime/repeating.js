// `iter::repeat(value)`: a clone of `value` each step, and `value` dropped,
// `drop`, when it's closed, JS's `return()`, as Rust drops it with its
// iterator; JS's `take` and `drop` close what they step, once (ADR 0422).
// Its hint std's, `(usize::MAX, None)` (#246).
function $repeating(value, clone = (value) => value, drop) {
  const it = Iterator.from({
    next() {
      return { done: false, value: clone(value) };
    },
    return() {
      drop?.(value);
      return { done: true, value: undefined };
    },
  });
  it.hint = () => [2 ** 32 - 1, undefined];
  return it;
}

// `iter::repeat_n(value, count)`: a clone of `value` each step but the
// last, which is `value` itself, as std's moves it; of a count of none,
// `value` dropped at once, and else when it's closed before its last
// (ADR 0513). Its hint exact, what it has left, as std's (#246).
function $repeatingN(value, count, clone = (value) => value, drop) {
  let left = count;
  if (left === 0) drop?.(value);
  const it = Iterator.from({
    next() {
      if (left === 0) return { done: true, value: undefined };
      left--;
      return { done: false, value: left === 0 ? value : clone(value) };
    },
    return() {
      if (left > 0) {
        left = 0;
        drop?.(value);
      }
      return { done: true, value: undefined };
    },
  });
  it.hint = () => [left, left];
  return it;
}
