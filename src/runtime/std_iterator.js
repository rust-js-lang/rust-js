// A std struct whose `Iterator` impl is compiled into `@rust-js/std`, a
// `Peekable`'s `{ iter, peeked }` (ADR 0561), as the JS iterator it is:
// itself, with `Iterator.prototype`'s helpers, its `next` the compiled one,
// `next`, which gives `Some` of an item boxed (ADR 0051), or `None` at its
// end, as `$iterator` steps the crate's own.
function $stdIterator(struct, next) {
  const it = Object.assign(Object.create(Iterator.prototype), struct);
  it.next = () => {
    const item = next(it);
    return item === undefined ? { done: true, value: undefined } : { done: false, value: $someValue(item) };
  };
  return it;
}
