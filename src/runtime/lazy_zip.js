// `a.zip(b)`, lazy: a pair of each's next, till either ends; an item of
// `a`'s stepped where `b`'s run out given to `dropA`, as Rust drops it, and
// `b` closed as it ends, which drops what it holds (ADR 0465).
function* $lazyZip(a, b, dropA) {
  const right = b[Symbol.iterator]();
  try {
    for (const item of a) {
      const other = right.next();
      if (other.done) {
        dropA?.(item);
        return;
      }
      yield [item, other.value];
    }
  } finally {
    right.return?.();
  }
}
