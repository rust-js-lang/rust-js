
class $KeyMap extends Map {
  constructor(entries) {
    super();
    if (entries) {
      for (const [key, value] of entries) {
        this.set(key, value);
      }
    }
  }
  // The string an entry is kept under: its key's value (ADR 0121).
  keyOf(key) {
    return $key(key);
  }
  get(key) {
    const entry = super.get(this.keyOf(key));
    return entry === undefined ? undefined : entry[1];
  }
  set(key, value) {
    const found = this.keyOf(key);
    const entry = super.get(found);
    if (entry === undefined) {
      super.set(found, [key, value]);
    } else {
      entry[1] = value;
    }
    return this;
  }
  has(key) {
    return super.has(this.keyOf(key));
  }
  delete(key) {
    return super.delete(this.keyOf(key));
  }
  *entries() {
    for (const [key, value] of super.values()) {
      yield [key, value];
    }
  }
  *keys() {
    for (const [key] of super.values()) {
      yield key;
    }
  }
  *values() {
    for (const [, value] of super.values()) {
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
