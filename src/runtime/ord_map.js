// A B-tree of keys it finds by their own `Ord` (ADR 0404): its entries
// kept in `cmp`'s order, each found by a binary search, so a key `cmp`
// says is `Equal` to one it holds finds that one, as std's B-tree does.
class $OrdMap extends Map {
  #entries = [];
  constructor(cmp, entries) {
    super();
    this.cmp = cmp;
    if (entries) {
      for (const [key, value] of entries) {
        this.set(key, value);
      }
    }
  }
  // Where `key` is, or would go: its index, and whether it's there.
  at(key) {
    let low = 0;
    let high = this.#entries.length;
    while (low < high) {
      const mid = (low + high) >>> 1;
      const order = this.cmp(this.#entries[mid][0], key);
      if (order === 0) return [mid, true];
      if (order < 0) low = mid + 1;
      else high = mid;
    }
    return [low, false];
  }
  get size() {
    return this.#entries.length;
  }
  get(key) {
    const [i, there] = this.at(key);
    return there ? this.#entries[i][1] : undefined;
  }
  set(key, value) {
    const [i, there] = this.at(key);
    if (there) {
      this.#entries[i][1] = value;
    } else {
      this.#entries.splice(i, 0, [key, value]);
    }
    return this;
  }
  has(key) {
    return this.at(key)[1];
  }
  // The key it holds equal to `key` (ADR 0467).
  storedKey(key) {
    const [i, there] = this.at(key);
    return there ? this.#entries[i][0] : undefined;
  }
  delete(key) {
    const [i, there] = this.at(key);
    if (there) this.#entries.splice(i, 1);
    return there;
  }
  clear() {
    this.#entries = [];
  }
  *entries() {
    for (const [key, value] of this.#entries) {
      yield [key, value];
    }
  }
  *keys() {
    for (const [key] of this.#entries) {
      yield key;
    }
  }
  *values() {
    for (const [, value] of this.#entries) {
      yield value;
    }
  }
  [Symbol.iterator]() {
    return this.entries();
  }
  forEach(f, that) {
    for (const [key, value] of this.entries()) {
      f.call(that, value, key, this);
    }
  }
}

class $OrdSet extends Set {
  #items;
  constructor(cmp, items) {
    super();
    this.#items = new $OrdMap(cmp);
    if (items) {
      for (const item of items) {
        this.add(item);
      }
    }
  }
  get size() {
    return this.#items.size;
  }
  add(item) {
    if (!this.#items.has(item)) {
      this.#items.set(item, item);
    }
    return this;
  }
  has(item) {
    return this.#items.has(item);
  }
  storedKey(item) {
    return this.#items.storedKey(item);
  }
  delete(item) {
    return this.#items.delete(item);
  }
  clear() {
    this.#items.clear();
  }
  values() {
    return this.#items.keys();
  }
  keys() {
    return this.#items.keys();
  }
  *entries() {
    for (const item of this.#items.keys()) {
      yield [item, item];
    }
  }
  [Symbol.iterator]() {
    return this.#items.keys();
  }
  forEach(f, that) {
    for (const item of this.#items.keys()) {
      f.call(that, item, item, this);
    }
  }
}

// The class of a B-tree ordered by `cmp`, which `new` makes as `Map` is
// made, of its entries or none, and which a copy of one makes again.
function $ordMap(cmp) {
  return class extends $OrdMap {
    constructor(entries) {
      super(cmp, entries);
    }
  };
}

function $ordSet(cmp) {
  return class extends $OrdSet {
    constructor(items) {
      super(cmp, items);
    }
  };
}
