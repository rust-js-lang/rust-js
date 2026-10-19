// `it.fuse()`, Rust's `Fuse`: `iter` stepped till it ends, then never
// again. Of one that isn't a `FusedIterator`, not `fused`, `iter` is closed
// as it ends, which drops it as std's does, and `undefined` then, as its
// `Debug` shows it; a `FusedIterator`'s, which stays ended, is kept, and
// stepped again, as std's is (ADR 0473).
function $fuse(iter, fused = false) {
  const it = Iterator.from({
    next() {
      if (it.iter === undefined) {
        return { done: true, value: undefined };
      }
      const step = it.iter.next();
      if (step.done && !fused) {
        it.iter.return?.();
        it.iter = undefined;
      }
      return step;
    },
    return() {
      it.iter?.return?.();
      it.iter = undefined;
      return { done: true, value: undefined };
    },
  });
  it.iter = iter;
  it.fused = fused;
  return it;
}
