// `skip(n)` of an iterator that owns items with a destructor, moved into
// it: the first `n` dropped by `drop` as it's first stepped, as Rust's `nth`
// drops what it skips, and what it hasn't given dropped when it's closed,
// as `$closing`'s is (ADR 0422).
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
      for (let step = it.next(); !step.done; step = it.next()) drop(step.value);
      return { done: true, value: undefined };
    },
  });
}
