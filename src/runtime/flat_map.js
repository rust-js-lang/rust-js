// `it.flat_map(f)`, or `flatten()`, of a source and inner iterators that
// both hold what has a destructor: JS's `flatMap`, but closed as Rust drops
// a `FlatMap`, its source first, then the inner iterator it's stepping,
// each `f` made (ADR 0510).
function* $flatMap(it, f) {
  let inner;
  let ended = false;
  try {
    while (true) {
      const outer = it.next();
      if (outer.done) {
        ended = true;
        return;
      }
      inner = f(outer.value);
      for (let step = inner.next(); !step.done; step = inner.next()) yield step.value;
      inner = undefined;
    }
  } finally {
    if (!ended) it.return?.();
    inner?.return?.();
  }
}
