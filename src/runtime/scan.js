// `it.scan(init, f)` of a JS iterator, lazy as Rust's: each `Some` `f`
// gives, given its state's box and an item, till its first `None`, which
// ends it, closing what it steps, as `take_while`'s end does (ADR 0517).
function* $lazyScan(items, init, f) {
  const state = { value: init };
  for (const item of items) {
    const next = f(state, item);
    if (next === undefined) return;
    yield next;
  }
}
