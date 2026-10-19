
// A Rust iterator of the crate's own as a JS one: its `next` returns the
// item, or `None` at the end. JS's iterator helpers are lazy, like Rust's
// adapters, so an endless one is fine until something wants all of it. A
// generic `next` may box its `Some` (ADR 0051): `boxed` unboxes it. `of`
// is what it's of, which its drop drops (ADR 0421). Given its `nextBack`,
// it knows its back, which `$nextBack` steps; given its `len`, its length,
// an `ExactSizeIterator`'s (ADR 0456).
function $iterator(iterator, next, boxed = false, nextBack, len) {
  const step = (item) => (item == null ? { done: true, value: undefined } : { done: false, value: boxed ? $someValue(item) : item });
  const it = Iterator.from({
    next() {
      return step(next(iterator));
    }
  });
  it.of = iterator;
  // Another of a value of its type, stepped as it is: a clone's (ADR 0473).
  it.with = (value) => $iterator(value, next, boxed, nextBack, len);
  if (nextBack !== undefined) {
    it.back = () => step(nextBack(iterator));
  }
  if (len !== undefined) {
    it.len = () => len(iterator);
  }
  return it;
}
