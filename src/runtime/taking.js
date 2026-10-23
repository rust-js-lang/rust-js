// `it.take(n)` explicit, Rust's `Take` (ADR 0542): what it steps, `iter`,
// and how many it has left to take, `n`, kept, which a clone reads. Its
// back is the last of those it has left, the ones past it dropped by
// `drop` where items have a destructor, as Rust's `next_back` steps its
// source's `nth_back`; its length the fewer of its source's and `n`.
function $taking(iter, n, drop) {
  const taking = new $Steps(
    () => {
      if (taking.n === 0) return { done: true, value: undefined };
      taking.n--;
      return taking.iter.next();
    },
    () => $closed(taking.iter),
  );
  taking.iter = iter;
  taking.n = n;
  taking.hint = () => {
    const [low, high] = $sizeHint(taking.iter);
    return [Math.min(low, taking.n), high === undefined ? taking.n : Math.min(high, taking.n)];
  };
  taking.back = () => {
    if (taking.n === 0) return { done: true, value: undefined };
    const past = Math.max(taking.iter.len() - taking.n, 0);
    taking.n--;
    for (let k = 0; k < past; k++) {
      const step = taking.iter.back();
      if (step.done) return step;
      drop?.(step.value);
    }
    return taking.iter.back();
  };
  taking.len = () => Math.min(taking.iter.len(), taking.n);
  return taking;
}
