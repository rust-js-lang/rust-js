// An iterator that owns its items, given to a `take`, which closes what it
// steps as it ends, or is dropped: each item it hasn't given dropped by
// `drop`, as Rust's drops them with it (ADR 0422).
function $closing(it, drop) {
  return Iterator.from({
    next() {
      return it.next();
    },
    return() {
      for (let step = it.next(); !step.done; step = it.next()) drop(step.value);
      return { done: true, value: undefined };
    },
  });
}
