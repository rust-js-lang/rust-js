// `it.scan(init, f)` of a JS iterator, lazy as Rust's: each `Some` `f`
// gives, given its state and an item; a `None` ends what's stepped now,
// closing what it steps, which drops what's left of one with a destructor
// (`$closing`), as Rust's drop of it does, and a step after steps on, as
// Rust's `Scan`, which isn't fused, does, of an array's (ADR 0517). The state's `&mut` is a box of it where it's a `handle`, a
// number's, else the state itself, an object `f` changes in place.
function $lazyScan(items, init, f, handle = true) {
  const state = handle ? { value: init } : init;
  const source = items[Symbol.iterator]();
  return new $Steps(
    () => {
      const step = source.next();
      const next = step.done ? undefined : f(state, step.value);
      if (next !== undefined) return { done: false, value: next };
      source.return?.();
      return { done: true, value: undefined };
    },
    () => source.return?.(),
  );
}
