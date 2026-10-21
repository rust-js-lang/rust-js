// A lazy iterator made `peekable()` (ADR 0164): a JS iterator that steps
// it only when asked, and keeps the step `peek` took for the next `next`,
// as Rust's `Peekable` does. One of an array's items is its `$iter`, which
// peeks at its array. Of items with a destructor, `drop`'s: closed, its
// source first, then what it peeked at, as Rust drops a `Peekable`'s
// fields (ADR 0514). What it steps is `iter`, as an explicit adapter's,
// whose `Debug` and clone read it and `peeked` (ADR 0543).
class $Peekable extends Iterator {
  constructor(it, drop) {
    super();
    this.iter = it;
    this.peeked = undefined;
    this.drop = drop;
  }
  next() {
    const step = this.peeked ?? this.iter.next();
    this.peeked = undefined;
    return step;
  }
  peek() {
    this.peeked ??= this.iter.next();
    return this.peeked;
  }
  // Its last, `next_back()`'s, as Rust's: what it peeked at, once what it
  // steps has none left.
  back() {
    if (this.peeked === undefined) return this.iter.back();
    if (this.peeked.done) return this.peeked;
    const step = this.iter.back();
    if (!step.done) return step;
    const peeked = this.peeked;
    this.peeked = undefined;
    return peeked;
  }
  // Its size hint, Rust's: what it steps', and what it peeked at, an end
  // none (ADR 0543).
  hint() {
    if (this.peeked?.done) return [0, 0];
    const more = this.peeked === undefined ? 0 : 1;
    const [low, high] = $sizeHint(this.iter);
    return [low + more, high === undefined ? undefined : high + more];
  }
  len() {
    return (this.peeked === undefined || this.peeked.done ? 0 : 1) + this.iter.len();
  }
  return() {
    this.iter.return?.();
    const peeked = this.peeked;
    this.peeked = undefined;
    if (peeked !== undefined && !peeked.done) this.drop?.(peeked.value);
    return { done: true, value: undefined };
  }
}
function $peekable(it, drop) {
  return new $Peekable(it, drop);
}

// A clone of `peekable`, where it is (ADR 0543): of `iter`, a clone of what
// it steps, and of what it peeked at, an item's clone, `clone`, made.
function $peekableClone(peekable, iter, clone) {
  const copy = new $Peekable(iter, peekable.drop);
  const peeked = peekable.peeked;
  copy.peeked = peeked === undefined || peeked.done ? peeked : { done: false, value: clone(peeked.value) };
  return copy;
}

// What `peekable` peeked at, an item, for its `Debug` (ADR 0543).
function $peekedItem(peekable) {
  return peekable.peeked.value;
}
