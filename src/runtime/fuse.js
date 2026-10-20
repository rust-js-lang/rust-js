// `it.fuse()`, Rust's `Fuse`: `iter` stepped till it ends, then never
// again. Of one that isn't a `FusedIterator`, not `fused`, `iter` is closed
// as it ends, which drops it as std's does, and `undefined` then, as its
// `Debug` shows it; a `FusedIterator`'s, which stays ended, is kept, and
// stepped again, as std's is (ADR 0473). Of one that knows its back or its
// length, it knows them too, as Rust's `Fuse` does (ADR 0528).
function $fuse(iter, fused = false) {
  const stepped = (step) => () => {
    if (it.iter === undefined) {
      return { done: true, value: undefined };
    }
    const at = step();
    if (at.done && !fused) {
      it.iter.return?.();
      it.iter = undefined;
    }
    return at;
  };
  const it = Iterator.from({
    next: stepped(() => it.iter.next()),
    return() {
      it.iter?.return?.();
      it.iter = undefined;
      return { done: true, value: undefined };
    },
  });
  it.iter = iter;
  it.fused = fused;
  if (iter.back !== undefined) it.back = stepped(() => it.iter.back());
  if (iter.len !== undefined) it.len = () => (it.iter === undefined ? 0 : it.iter.len());
  return it;
}
