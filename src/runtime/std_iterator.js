// A std struct whose `Iterator` impl is compiled into `@rust-js/std`, a
// `Peekable`'s `{ iter, peeked }` or a `Fuse`'s `{ iter }` (ADR 0561), as
// the JS iterator it is: itself, with `Iterator.prototype`'s helpers, its
// `next` the compiled one, `steps.next`, which gives `Some` of an item boxed
// (ADR 0051), or `None` at its end, as `$iterator` steps the crate's own;
// its size hint `steps.hint`, the compiled `size_hint` or a rewrite of one
// whose body reaches memory, `$peekableHint`. It keeps `steps` as `$steps`,
// a name no field has, for its clone to step as it does. Its back and its
// length a use site gives, of its compiled `next_back` and `len` with that
// use's dictionaries, `$knowing`'s. It's a `$Steps` (ADR 0669), whose fold,
// a peekable's or a fuse's, is std's, `steps.fold`, its source's (#162);
// closing it closes nothing, but of a kind dropped by closing, a `Rev`'s,
// whose `steps.close` drops what it holds.
function $stdIterator(struct, steps) {
  const it = Object.assign(Object.create($Steps.prototype), struct);
  const step = (item) => (item === undefined ? { done: true, value: undefined } : { done: false, value: $someValue(item) });
  it.$steps = steps;
  it.next = () => step(steps.next(it));
  // Closed, of a kind dropped by closing, `Rev`'s: what it holds dropped,
  // once, `steps.close`'s (ADR 0871); another's closes nothing.
  it.return = () => {
    if (steps.close !== undefined && !it.$closed) {
      it.$closed = true;
      steps.close(it);
    }
    return { done: true, value: undefined };
  };
  // What std's own compiled methods take whole, `fold` and `count`, as
  // Rust's move it, dropping what it held there: closing it after drops
  // nothing again (ADR 0912).
  const taking = (method, made) => {
    if (steps.takes?.includes(method)) it.$closed = true;
    return made;
  };
  if (steps.fold !== undefined) {
    it.fold = (init, f) => taking("fold", steps.fold(it, init, f));
  }
  // Its `try_fold`, std's, where it passes it on to its source's (#214).
  if (steps.tryFold !== undefined) {
    it.tryFold = (init, f, broke) => steps.tryFold(it, init, f, broke);
  }
  if (steps.hint !== undefined) {
    it.hint = () => steps.hint(it);
  }
  // Its `count`, std's, its source's own (#293).
  if (steps.count !== undefined) {
    it.count = (drop) => taking("count", steps.count(it, drop));
  }
  // Its `advance_by`, std's, its source's, which drops what it passes as
  // its source does, a `Vec`'s as one slice (#286).
  if (steps.advanceBy !== undefined) {
    it.advanceBy = (m, drop) => steps.advanceBy(it, m, drop);
  }
  // Its `nth`, the compiled one, which passes it on to its source's (#182).
  // Each std's body, which drops what it passes itself, as `$iterator`'s
  // own do: `ownNth`, `ownLast`.
  if (steps.nth !== undefined) {
    it.nth = (k) => step(steps.nth(it, k));
    it.ownNth = true;
  }
  // And its `last`, the compiled one, its source's (#182).
  if (steps.last !== undefined) {
    it.last = () => step(steps.last(it));
    it.ownLast = true;
  }
  it.knowing = (back, len, rfold, nthBack) => {
    if (back !== undefined) it.back = () => step(back(it));
    if (len !== undefined) it.len = () => len(it);
    if (rfold !== undefined) it.rfold = (init, f) => rfold(it, init, f);
    if (nthBack !== undefined) it.nthBack = (k) => step(nthBack(it, k));
  };
  return it;
}
