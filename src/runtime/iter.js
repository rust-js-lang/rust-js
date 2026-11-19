// What marks an array's iterator, `$iter`'s, which a clone, `$iter` of its
// items again, is too: the one answer of whether an iterator is one, so its
// `items`, `at` and `end` may be read, never read of whether it has an
// `items` field, as a crate's struct may (#2159).
const $arrayIterKey = Symbol("rust-js.arrayIter");
function $arrayIter(it) {
  return it != null && it[$arrayIterKey] === true;
}

// A JS iterator of an array that knows where it is (ADR 0071): an
// `Iterator`, whose helpers, `map` and the like, step it as Rust's adapters
// do. Of an iterator std hints no lower bound of, `lowerless`, a B-tree's
// `range(..)`, its hint std's lower bound, 0, of what's left (#1215).
function $iter(items, lowerless) {
  // One already, a variable's that holds its own `$iter`: itself, where it
  // is, which an array's iterator given on may be (ADRs 0429, 0456).
  if (!Array.isArray(items) && items.back !== undefined) return items;
  // Of what's neither, an iterator that doesn't know its back, a compiler's
  // mistake: loud, never read as an empty array (#1292). A `TypeError`, so
  // no `catch_unwind` takes it for a panic.
  if (typeof items.length !== "number") throw new TypeError("rust-js: an iterator stepped as an array's, which doesn't know its back");
  const it = Object.create(Iterator.prototype);
  it.items = items;
  it[$arrayIterKey] = true;
  it.at = 0;
  // Where its back is, which `next_back()` steps down.
  it.end = items.length;
  it.next = function () {
    return this.at < this.end ? { value: this.items[this.at++], done: false } : { value: undefined, done: true };
  };
  // Its last item, as `next()` gives its first: what `$nextBack` steps.
  it.back = function () {
    return this.at < this.end ? { value: this.items[--this.end], done: false } : { value: undefined, done: true };
  };
  // How many are left, `len()` of an `ExactSizeIterator` (ADR 0456).
  it.len = function () {
    return this.end - this.at;
  };
  // Of `this`, so a clone's is its own (ADR 0181).
  if (lowerless) {
    it.sizeHint = function () {
      return [0, this.end - this.at];
    };
  }
  return it;
}

