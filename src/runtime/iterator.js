
// A Rust iterator of the crate's own as a JS one: its `next` returns the
// item, or `None` at the end. JS's iterator helpers are lazy, like Rust's
// adapters, so an endless one is fine until something wants all of it. A
// generic `next` may box its `Some` (ADR 0051): `boxed` unboxes it. `of`
// is what it's of, which its drop drops (ADR 0421). Given its `nextBack`,
// it knows its back, which `$nextBack` steps; given its `len`, its length,
// an `ExactSizeIterator`'s (ADR 0456).
function $iterator(iterator, next, boxed = false, nextBack, len, hint) {
  const step = (item) => (item == null ? { done: true, value: undefined } : { done: false, value: boxed ? $someValue(item) : item });
  const it = Iterator.from({
    next() {
      return step(next(iterator));
    }
  });
  it.of = iterator;
  // Another of a value of its type, stepped as it is: a clone's (ADR 0473).
  it.with = (value) => $iterator(value, next, boxed, nextBack, len, hint);
  if (nextBack !== undefined) {
    it.back = () => step(nextBack(iterator));
  }
  if (len !== undefined) {
    it.len = () => len(iterator);
  }
  // Its own `size_hint`, the crate's impl's, which `$sizeHint` reads.
  if (hint !== undefined) {
    it.hint = () => hint(iterator);
  }
  // Its back and its length, a use site's, where they weren't given where
  // it was made, of the value it steps (ADR 0542).
  it.knowing = (back, size) => {
    if (back !== undefined) it.back = () => step(back(iterator));
    if (size !== undefined) it.len = () => size(iterator);
  };
  return it;
}

// `it`, an explicit adapter (ADR 0542), stepped by a use site that knows its
// source's back and length, `back` and `len`, functions of the value the
// `$iterator` it steps steps, which that's given: what it steps from its
// back as Rust's does. Or a compiled std iterator's struct (ADR 0561),
// `$stdIterator`'s, given its own.
function $knowing(it, back, len) {
  let source = it;
  while (source.knowing === undefined && source.iter !== undefined) source = source.iter;
  source.knowing?.(back, len);
  return it;
}
