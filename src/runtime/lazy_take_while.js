
// `it.take_while(keep)`: each item `keep` holds of; the first it doesn't
// ends it for good, closing what it steps, as Rust's flag does, and a
// `None` of its source ends what's stepped now, as Rust's passes it on.
function $lazyTakeWhile(items, keep) {
  const source = items[Symbol.iterator]();
  let ended = false;
  return new $Steps(
    () => {
      if (ended) return { done: true, value: undefined };
      const step = source.next();
      if (step.done || keep(step.value)) return step;
      ended = true;
      source.return?.();
      return { done: true, value: undefined };
    },
    () => source.return?.(),
  );
}

// `it.map_while(f)`: each of what `f` gives that `present` keeps; one it
// doesn't ends what's stepped now, closing what it steps, as `$lazyScan`'s
// `None` does, and a step after steps on, as Rust's `MapWhile`, which
// isn't fused, does, where `take_while` stays ended.
function $lazyMapWhile(items, f, present) {
  const source = items[Symbol.iterator]();
  return new $Steps(
    () => {
      const step = source.next();
      const value = step.done ? undefined : f(step.value);
      if (!step.done && present(value)) return { done: false, value };
      source.return?.();
      return { done: true, value: undefined };
    },
    () => source.return?.(),
  );
}
