function $iter(items) {
  return {
    items,
    at: 0,
    // Where its back is, which `next_back()` steps down.
    end: items.length,
    next() {
      return this.at < this.end ? { value: this.items[this.at++], done: false } : { value: undefined, done: true };
    },
    [Symbol.iterator]() {
      return this;
    },
  };
}
