// `it.flat_map(f)`, or `flatten()`, of a source and inner iterators that
// both hold what has a destructor: JS's `flatMap`, but closed as Rust drops
// a `FlatMap`, its source first, then the inner iterator it's stepping,
// each `f` made (ADR 0510). A `$Steps`, not a generator, which would close
// both as a step throws, a panic in `f`: what takes it closes it, after
// what it holds itself, a `last`'s kept item, as Rust's frames unwind
// (#426).
function $flatMap(it, f) {
  let inner;
  let ended = false;
  return new $Steps(
    () => {
      for (;;) {
        if (inner !== undefined) {
          const step = inner.next();
          if (!step.done) return step;
          inner = undefined;
        }
        const outer = it.next();
        if (outer.done) {
          ended = true;
          return outer;
        }
        inner = f(outer.value);
      }
    },
    () => {
      if (!ended) {
        ended = true;
        it.return?.();
      }
      inner?.return?.();
      inner = undefined;
    },
  );
}
