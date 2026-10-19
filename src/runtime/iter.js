// A JS iterator of an array that knows where it is (ADR 0071): an
// `Iterator`, whose helpers, `map` and the like, step it as Rust's adapters
// do.
function $iter(items) {
  // One already, a variable's that holds its own `$iter`: itself, where it
  // is, which an array's iterator given on may be (ADRs 0429, 0456).
  if (!Array.isArray(items) && items.back !== undefined) return items;
  const it = Object.create(Iterator.prototype);
  it.items = items;
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
  return it;
}
