// `it.scan(init, f)` of a JS iterator, lazy as Rust's: each `Some` `f`
// gives, given its state and an item, till its first `None`, which ends
// it, closing what it steps, as `take_while`'s end does (ADR 0517). The
// state's `&mut` is a box of it where it's a `handle`, a number's, else
// the state itself, an object `f` changes in place.
function* $lazyScan(items, init, f, handle = true) {
  const state = handle ? { value: init } : init;
  for (const item of items) {
    const next = f(state, item);
    if (next === undefined) return;
    yield next;
  }
}
