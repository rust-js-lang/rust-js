// A B-tree of keys it finds by their own `Ord` (ADR 0404): its entries
// kept in `cmp`'s order, each found by one search, so a key `cmp` says is
// `Equal` to one it holds finds that one, as std's B-tree does. The search
// asks `cmp(key, held)`, the key given against each it holds, as std's
// does; of up to eleven entries, one node of std's, each from the first in
// turn, as std's search of a node, so a `cmp` that prints or panics sees
// what std's sees. Past eleven, std's nodes are another shape, which this
// isn't: a halving search, in std's argument order (#911).
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
    const entries = this.#entries;
    if (entries.length <= 11) {
      for (let i = 0; i < entries.length; i++) {
        const order = this.cmp(key, entries[i][0]);
        if (order === 0) return [i, true];
        if (order < 0) return [i, false];
      }
      return [entries.length, false];
    }
    let low = 0;
    let high = entries.length;
    while (low < high) {
      const mid = (low + high) >>> 1;
      const order = this.cmp(key, entries[mid][0]);
      if (order === 0) return [mid, true];
      if (order > 0) low = mid + 1;
      else high = mid;
    }
    return [low, false];
  }
  // One search's answer, `$found`'s: whether `key` is there, the key it
  // holds equal to it and its value, and where.
  found(key) {
    const [i, there] = this.at(key);
    const entry = there ? this.#entries[i] : undefined;
    return { there, key: entry?.[0], value: entry?.[1], i };
  }
  // What a search found, `found`, given `value`, or `key` and `value` put
  // where it would go; or taken out; or those from it on taken out,
  // `split_off`'s: no search again.
  putFound(found, key, value) {
    if (found.there) this.#entries[found.i][1] = value;
    else this.#entries.splice(found.i, 0, [key, value]);
  }
  deleteFound(found) {
    if (found.there) this.#entries.splice(found.i, 1);
  }
  // The entry of `key`, one it holds, taken out by what it is, asking
  // nothing, as std's `retain` takes out what it walks to (#911).
  deleteHeld(key) {
    const i = this.#entries.findIndex(([held]) => held === key);
    if (i >= 0) this.#entries.splice(i, 1);
  }
  // Each key of `entries`, some it holds, with a handle on its value,
  // `$mutEntries`', each on the entry itself, asking nothing.
  entryHandles(entries) {
    const of = new Map(this.#entries.map((entry) => [entry[0], entry]));
    return Array.from(entries, ([key]) => {
      const entry = of.get(key);
      return [
        key,
        {
          get value() {
            return entry[1];
          },
          set value(item) {
            entry[1] = item;
          },
        },
      ];
    });
  }
  splitFound(found) {
    // A set's own map is an `$OrdMap` of its `cmp`, a map's of its class.
    const rest = this.constructor === $OrdMap ? new $OrdMap(this.cmp) : new this.constructor();
    rest.#entries = this.#entries.splice(found.i);
    return rest;
  }
  get size() {
    return this.#entries.length;
  }
  get(key) {
    return this.found(key).value;
  }
  set(key, value) {
    this.putFound(this.found(key), key, value);
    return this;
  }
  has(key) {
    return this.at(key)[1];
  }
  // The key it holds equal to `key` (ADR 0467).
  storedKey(key) {
    return this.found(key).key;
  }
  delete(key) {
    const found = this.found(key);
    this.deleteFound(found);
    return found.there;
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

// A set of items it finds by their own `Ord`: an `$OrdMap` of each to
// itself, each of its calls one search.
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
  found(item) {
    return this.#items.found(item);
  }
  putFound(found, item) {
    // One equal to it held stays: only its value, unread, is written.
    this.#items.putFound(found, item, item);
  }
  deleteFound(found) {
    this.#items.deleteFound(found);
  }
  deleteHeld(item) {
    this.#items.deleteHeld(item);
  }
  splitFound(found) {
    const rest = new this.constructor();
    rest.#items = this.#items.splitFound(found);
    return rest;
  }
  add(item) {
    this.putFound(this.found(item), item);
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
