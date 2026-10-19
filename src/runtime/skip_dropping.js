// `skip(n)` of items with a destructor: the first `n` dropped by `drop` as
// it's first stepped, as Rust's `nth` drops what it skips; closing it closes
// what it steps, which drops what it holds (ADRs 0422, 0465).
function $skipDropping(it, n, drop) {
  let skipping = n;
  return Iterator.from({
    next() {
      for (; skipping > 0; skipping--) {
        const step = it.next();
        if (step.done) {
          skipping = 0;
          return step;
        }
        drop(step.value);
      }
      return it.next();
    },
    return() {
      it.return?.();
      return { done: true, value: undefined };
    },
  });
}
