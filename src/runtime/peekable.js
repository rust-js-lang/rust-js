// A lazy iterator made `peekable()` (ADR 0164): a JS iterator that steps
// it only when asked, and keeps the step `peek` took for the next `next`,
// as Rust's `Peekable` does. One of an array's items is its `$iter`, which
// peeks at its array. Of items with a destructor, `drop`'s: closed, its
// source first, then what it peeked at, as Rust drops a `Peekable`'s
// fields (ADR 0514).
class $Peekable extends Iterator {
  constructor(it, drop) {
    super();
    this.it = it;
    this.peeked = undefined;
    this.drop = drop;
  }
  next() {
    const step = this.peeked ?? this.it.next();
    this.peeked = undefined;
    return step;
  }
  peek() {
    this.peeked ??= this.it.next();
    return this.peeked;
  }
  // Its last, `next_back()`'s, as Rust's: what it peeked at, once what it
  // steps has none left.
  back() {
    if (this.peeked === undefined) return this.it.back();
    if (this.peeked.done) return this.peeked;
    const step = this.it.back();
    if (!step.done) return step;
    const peeked = this.peeked;
    this.peeked = undefined;
    return peeked;
  }
  len() {
    return (this.peeked === undefined || this.peeked.done ? 0 : 1) + this.it.len();
  }
  return() {
    this.it.return?.();
    const peeked = this.peeked;
    this.peeked = undefined;
    if (peeked !== undefined && !peeked.done) this.drop?.(peeked.value);
    return { done: true, value: undefined };
  }
}
function $peekable(it, drop) {
  return new $Peekable(it, drop);
}
