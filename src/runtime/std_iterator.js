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
  // What std's own compiled methods take whole, `fold`, `count` and `last`, as
  // Rust's move it, dropping what it held there: closing it after drops
  // nothing again (ADR 0912). Taken once the call ends, though it throws:
  // one that panics drops what it took as it unwinds, so a close after it,
  // a generic caller's, drops nothing twice (rd764); not before, as a
  // compiled body may close it itself, `Take`'s `fold`.
  const taking = (method, run) => {
    try {
      return run();
    } finally {
      if (steps.takes?.includes(method)) it.$closed = true;
    }
  };
  if (steps.hint !== undefined) {
    it.hint = () => steps.hint(it);
  }
  // Each its compiled body's, which passes it on as std's does: its `fold`,
  // given `unwind`, the compiled fold's accumulator's drop, where a stage's
  // panic strands it (#424); its `try_fold`, where it passes it on to its
  // source's (#214); its `count`, its source's own (#293); its `advance_by`,
  // its source's, which drops what it passes as its source does, a `Vec`'s
  // as one slice (#286), or std's compiled one, its `Result` how many there
  // weren't, as `advanceBy` gives it (ADR 0913); its `nth` and its `last`,
  // which pass it on to its source's (#182). Each std's body, which drops
  // what it passes itself, as `$iterator`'s own do: `ownNth`, `ownLast`.
  const advance =
    steps.advance &&
    ((m) => {
      const advanced = steps.advance(it, m);
      return advanced.TAG === "Ok" ? 0 : advanced._0;
    });
  $forward(it, steps, {
    // What drops with `f`, `ends`, its body's drop of its `F` (#710).
    fold: steps.fold && ((init, f, unwind, ends) => taking("fold", () => steps.fold(it, init, f, unwind, ends))),
    tryFold: steps.tryFold && ((init, f, broke, unwind) => steps.tryFold(it, init, f, broke, unwind)),
    count: steps.count && ((drop) => taking("count", () => steps.count(it, drop))),
    advanceBy: steps.advanceBy === undefined ? advance : (m, drop) => steps.advanceBy(it, m, drop),
    nth: steps.nth && ((k) => step(steps.nth(it, k))),
    last: steps.last && (() => step(taking("last", () => steps.last(it)))),
  });
  if (steps.nth !== undefined) it.ownNth = true;
  if (steps.last !== undefined) it.ownLast = true;
  it.knowing = (back, len, rfold, nthBack, tryRfold) => {
    if (back !== undefined) it.back = () => step(back(it));
    if (len !== undefined) it.len = () => len(it);
    // Its compiled `rfold` and `nth_back`, and `try_rfold`, which `rfind`
    // asks (ADR 0930). Its `rfold` takes it whole, as its `fold` does, once
    // the call ends, though it throws: a close after one that panicked
    // drops nothing twice (#649).
    $forward(it, steps, {
      rfold:
        rfold &&
        ((init, f, unwind, ends) => {
          try {
            return rfold(it, init, f, unwind, ends);
          } finally {
            it.$closed = true;
          }
        }),
      nthBack: nthBack && ((k) => step(nthBack(it, k))),
      tryRfold: tryRfold && ((init, f, broke, unwind) => tryRfold(it, init, f, broke, unwind)),
    });
  };
  // Its fields, the struct's, are what a move copies and what a write of
  // the whole replaces; its methods step `it` itself (#537). Moved out
  // apart, `$apart`'s: another of a copy of its fields, stepped as it is.
  // Written whole, `$assign`'s: its fields `value`'s, in place, and closed
  // where `value` is, as Rust's `*it = value` drops what it held, closing
  // it, and moves in `value`, which drops its own later. Its back and its
  // length are each use site's, `$knowing`'s, as they were.
  const fields = Object.keys(struct);
  const of = (value) => Object.fromEntries(fields.map((field) => [field, value[field]]));
  const closed = (target, value) => {
    target.$closed = value.$closed;
    return target;
  };
  it.$copy = () => closed($stdIterator(of(it), steps), it);
  it.$become = (value) => closed(Object.assign(it, of(value)), value);
  return it;
}
