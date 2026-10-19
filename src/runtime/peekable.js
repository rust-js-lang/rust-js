// A lazy iterator made `peekable()` (ADR 0164): a JS iterator that steps
// it only when asked, and keeps the step `peek` took for the next `next`,
// as Rust's `Peekable` does. One of an array's items is its `$iter`, which
// peeks at its array.
class $Peekable extends Iterator {
  constructor(it) {
    super();
    this.it = it;
    this.peeked = undefined;
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
}
function $peekable(it) {
  return new $Peekable(it);
}
