// A std struct whose `Iterator` impl is compiled into `@rust-js/std`, a
// `Peekable`'s `{ iter, peeked }` or a `Fuse`'s `{ iter }` (ADR 0561), as
// the JS iterator it is: itself, with `Iterator.prototype`'s helpers, its
// `next` the compiled one, `steps.next`, which gives `Some` of an item boxed
// (ADR 0051), or `None` at its end, as `$iterator` steps the crate's own;
// its size hint `steps.hint`, the compiled `size_hint` or a rewrite of one
// whose body reaches memory, `$peekableHint`. It keeps `steps` as `$steps`,
// a name no field has, for its clone to step as it does. Its back and its
// length a use site gives, of its compiled `next_back` and `len` with that
// use's dictionaries, `$knowing`'s.
function $stdIterator(struct, steps) {
  const it = Object.assign(Object.create(Iterator.prototype), struct);
  const step = (item) => (item === undefined ? { done: true, value: undefined } : { done: false, value: $someValue(item) });
  it.$steps = steps;
  it.next = () => step(steps.next(it));
  if (steps.hint !== undefined) {
    it.hint = () => steps.hint(it);
  }
  it.knowing = (back, len) => {
    if (back !== undefined) it.back = () => step(back(it));
    if (len !== undefined) it.len = () => len(it);
  };
  return it;
}
