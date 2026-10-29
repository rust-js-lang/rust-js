
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
  // The item it holds equal to `item`, which its drop is of (ADR 0467).
  storedKey(item) {
    return this.#items.get(this.keyOf(item));
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

// A set of items found by value that hold a type equal only to itself
// (ADR 1015, amended): each kept under `$key` of `parts(item)`.
function $keySet(parts) {
  return class extends $KeySet {
    keyOf(item) {
      return $key(parts(item));
    }
  };
}
