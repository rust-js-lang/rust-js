// A JS iterator of an array that knows where it is (ADR 0071): an
// `Iterator`, whose helpers, `map` and the like, step it as Rust's adapters
// do.
function $iter(items) {
  const it = Object.create(Iterator.prototype);
  it.items = items;
  it.at = 0;
  // Where its back is, which `next_back()` steps down.
  it.end = items.length;
  it.next = function () {
    return this.at < this.end ? { value: this.items[this.at++], done: false } : { value: undefined, done: true };
  };
  return it;
}
