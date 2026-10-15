
class $KeySet extends Set {
  #items = new Map();
  constructor(items) {
    super();
    if (items) {
      for (const item of items) {
        this.add(item);
      }
    }
  }
  // The string an item is kept under: its value (ADR 0121).
  keyOf(item) {
    return $key(item);
  }
  get size() {
    return this.#items.size;
  }
  add(item) {
    const found = this.keyOf(item);
    if (!this.#items.has(found)) {
      this.#items.set(found, item);
    }
    return this;
  }
  has(item) {
    return this.#items.has(this.keyOf(item));
  }
  delete(item) {
    return this.#items.delete(this.keyOf(item));
  }
  clear() {
    this.#items.clear();
  }
  values() {
    return this.#items.values();
  }
  keys() {
    return this.#items.values();
  }
  *entries() {
    for (const item of this.#items.values()) {
      yield [item, item];
    }
  }
  [Symbol.iterator]() {
    return this.#items.values();
  }
  forEach(f, that) {
    for (const item of this.#items.values()) {
      f.call(that, item, item, this);
    }
  }
}
