
function* $lazyTakeWhile(items, keep) {
  for (const item of items) {
    if (!keep(item)) {
      return;
    }
    yield item;
  }
}

// `it.map_while(f)`: each of what `f` gives that `present` keeps; one it
// doesn't ends what's stepped now, closing what it steps, as `$lazyScan`'s
// `None` does, and a step after steps on, as Rust's `MapWhile`, which
// isn't fused, does, where `take_while` stays ended.
function $lazyMapWhile(items, f, present) {
  const source = items[Symbol.iterator]();
  const it = Object.create(Iterator.prototype);
  it.next = () => {
    const step = source.next();
    const value = step.done ? undefined : f(step.value);
    if (!step.done && present(value)) return { done: false, value };
    source.return?.();
    return { done: true, value: undefined };
  };
  it.return = () => {
    source.return?.();
    return { done: true, value: undefined };
  };
  return it;
}
