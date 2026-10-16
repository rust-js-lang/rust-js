// `iter::repeat(value)`: a clone of `value` each step, and `value` dropped,
// `drop`, when it's closed, JS's `return()`, as Rust drops it with its
// iterator; JS's `take` and `drop` close what they step, once (ADR 0422).
function $repeating(value, clone = (value) => value, drop) {
  return Iterator.from({
    next() {
      return { done: false, value: clone(value) };
    },
    return() {
      drop?.(value);
      return { done: true, value: undefined };
    },
  });
}
