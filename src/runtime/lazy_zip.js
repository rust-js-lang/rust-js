// `a.zip(b)`, lazy: a pair of each's next, till either ends; an item of
// `a`'s stepped where `b`'s run out given to `dropA`, as Rust drops it;
// both closed as it's closed, which drops what they hold (ADR 0465). A
// step after its end steps each on, as Rust's `Zip`, which isn't fused,
// does.
function $lazyZip(a, b, dropA) {
  const left = a[Symbol.iterator]();
  const right = b[Symbol.iterator]();
  const close = () => {
    left.return?.();
    right.return?.();
  };
  const zipped = new $Steps(() => {
    const item = left.next();
    if (item.done) return item;
    const other = right.next();
    if (other.done) {
      dropA?.(item.value);
      return other;
    }
    return { done: false, value: [item.value, other.value] };
  }, close);
  zipped.from = [left, right];
  return zipped;
}

// `a.zip(b).rev()` of two iterators that know their backs and lengths,
// `$iter`s: the longer trimmed from its back to the shorter's length, then
// each's back paired, as Rust's `Zip::next_back` does (ADR 0493).
function* $zipBack(a, b) {
  for (let n = a.len() - b.len(); n > 0; n--) a.back();
  for (let n = b.len() - a.len(); n > 0; n--) b.back();
  while (true) {
    const x = a.back();
    const y = b.back();
    if (x.done || y.done) return;
    yield [x.value, y.value];
  }
}
