// `it.fuse()`, Rust's `Fuse`: `iter` stepped till it ends, then never
// again. Of one that isn't a `FusedIterator`, not `fused`, `iter` is closed
// as it ends, which drops it as std's does, and `undefined` then, as its
// `Debug` shows it; a `FusedIterator`'s, which stays ended, is kept, and
// stepped again, as std's is (ADR 0473). Its back and its length are what
// it steps', as Rust's `Fuse`'s are (ADR 0528), asked as they're stepped, as
// a use site may give what it steps them (ADR 0549).
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
  it.back = stepped(() => it.iter.back());
  it.len = () => (it.iter === undefined ? 0 : it.iter.len());
  return it;
}
