// `it.rev()` of a JS iterator that knows its back, a `$iter` or one closed
// to drop what it holds: it, stepped from its back, as Rust's `Rev` steps
// what it holds by `next_back`; its back its front, its length its own, and
// closed, `return()`, as it is, which drops what it hasn't given in its
// order, front to back (ADR 0516).
function $rev(it) {
  const rev = Object.create(Iterator.prototype);
  rev.next = () => it.back();
  rev.back = () => it.next();
  rev.len = () => it.len();
  rev.return = () => it.return?.() ?? { done: true, value: undefined };
  return rev;
}
