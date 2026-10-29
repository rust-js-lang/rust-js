// A B-tree of keys a hashed map finds (ADR 0404, amended): a map of
// `Base`, `Map` or one keyed by value, read in its keys' order, `cmp`'s,
// by Rust and by the JS it's given alike (#237). One added past its last
// is in place; one added before it sorts its entries again, in place,
// before it's next read, so adding stays a hashed map's, and reading in
// order once each change.
function $btreeMap(cmp, Base = Map) {
  return class extends Base {
    #last;
    #sorted = true;
    constructor(entries) {
      super();
      if (entries) {
        for (const [key, value] of entries) {
          this.set(key, value);
        }
      }
    }
    set(key, value) {
      if (this.#sorted && !this.has(key)) {
        if (this.size > 0 && cmp(this.#last, key) > 0) {
          this.#sorted = false;
        } else {
          this.#last = key;
        }
      }
      return super.set(key, value);
    }
    // A field's arrow, not a private method, which React's Babel can't
    // parse (ADR 0919, amended); of `Base`'s own methods, as `super` is.
    #order = () => {
      if (this.#sorted) return;
      const entries = Array.from(Base.prototype.entries.call(this)).sort((a, b) => cmp(a[0], b[0]));
      Base.prototype.clear.call(this);
      for (const [key, value] of entries) {
        Base.prototype.set.call(this, key, value);
      }
      this.#last = entries.at(-1)?.[0];
      this.#sorted = true;
    };
    entries() {
      this.#order();
      return super.entries();
    }
    keys() {
      this.#order();
      return super.keys();
    }
    values() {
      this.#order();
      return super.values();
    }
    [Symbol.iterator]() {
      this.#order();
      return super[Symbol.iterator]();
    }
    forEach(f, that) {
      this.#order();
      super.forEach(f, that);
    }
  };
}

// And a set's, of `Base`, `Set` or one keyed by value.
function $btreeSet(cmp, Base = Set) {
  return class extends Base {
    #last;
    #sorted = true;
    constructor(items) {
      super();
      if (items) {
        for (const item of items) {
          this.add(item);
        }
      }
    }
    add(item) {
      if (this.#sorted && !this.has(item)) {
        if (this.size > 0 && cmp(this.#last, item) > 0) {
          this.#sorted = false;
        } else {
          this.#last = item;
        }
      }
      return super.add(item);
    }
    #order = () => {
      if (this.#sorted) return;
      const items = $sort(Array.from(Base.prototype.values.call(this)), cmp);
      Base.prototype.clear.call(this);
      for (const item of items) {
        Base.prototype.add.call(this, item);
      }
      this.#last = items.at(-1);
      this.#sorted = true;
    };
    entries() {
      this.#order();
      return super.entries();
    }
    keys() {
      this.#order();
      return super.keys();
    }
    values() {
      this.#order();
      return super.values();
    }
    [Symbol.iterator]() {
      this.#order();
      return super[Symbol.iterator]();
    }
    forEach(f, that) {
      this.#order();
      super.forEach(f, that);
    }
  };
}

// Of keys std orders as `$cmp` does, numbers, strings and `char`s.
const $BTreeMap = $btreeMap($cmp);
const $BTreeSet = $btreeSet($cmp);
