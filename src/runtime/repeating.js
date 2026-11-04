// `iter::repeat(value)`: a clone of `value` each step, and `value` dropped,
// `drop`, when it's closed, JS's `return()`, once, as Rust drops it with its
// iterator (ADR 0422). A `$Steps`, whose `take` closes it though a clone
// throws, a panic, as Rust drops a `Take` that holds it as it unwinds,
// where JS's own `take` would be done with it (#708). Its hint std's,
// `(usize::MAX, None)` (#246).
function $repeating(value, clone = (value) => value, drop) {
  let closed = false;
  const it = new $Steps(
    () => ({ done: false, value: clone(value) }),
    () => {
      if (!closed) {
        closed = true;
        drop?.(value);
      }
    },
  );
  it.hint = () => [2 ** 32 - 1, undefined];
  return it;
}

// `iter::repeat_n(value, count)`: a clone of `value` each step but the
// last, which is `value` itself, as std's moves it; of a count of none,
// `value` dropped at once, and else when it's closed before its last
// (ADR 0513). A `$Steps`, as `$repeating` is (#708). Its hint exact, what
// it has left, as std's (#246).
function $repeatingN(value, count, clone = (value) => value, drop) {
  let left = count;
  if (left === 0) drop?.(value);
  const it = new $Steps(
    () => {
      if (left === 0) return { done: true, value: undefined };
      left--;
      return { done: false, value: left === 0 ? value : clone(value) };
    },
    () => {
      if (left > 0) {
        left = 0;
        drop?.(value);
      }
    },
  );
  it.hint = () => [left, left];
  return it;
}
