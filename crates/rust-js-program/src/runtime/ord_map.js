// A B-tree of keys it finds by their own `Ord` (ADR 0404): its entries
// kept in `cmp`'s order, each found by one search, so a key `cmp` says is
// `Equal` to one it holds finds that one, as std's B-tree does. The search
// asks `cmp(key, held)`, the key given against one it holds, as std's
// does, halving: of an order nothing sees, a quiet and total one, which
// asks what it asks unseen. One whose order can be seen is `$OrdTree`,
// std's nodes (#926).
//
// What a search found, `found(key)`'s, is what its writes are given, so
// that nothing searches twice: `putFound`, `deleteFound`, `replaceFound`
// and an entry's methods (`$entry`). Each entry is `[key, value]`, the
// one array wherever it's kept, so a handle on it (`entryHandles`) holds.
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
  // One made of `entries` in their order, each key once, a clone's of
  // `from`: kept as they are, asking nothing, as std's `clone_subtree`
  // asks nothing (#924); a tree's in `from`'s shape.
  static ofSorted(entries, from) {
    return new this().inOrder(entries, from);
  }
  inOrder(entries) {
    this.#entries = Array.from(entries, ([key, value]) => [key, value]);
    return this;
  }
  // One search's answer: whether `key` is there, the key it holds equal to
  // it, its value and its entry, and where.
  found(key) {
    const entries = this.#entries;
    let low = 0;
    let high = entries.length;
    while (low < high) {
      const mid = (low + high) >>> 1;
      const order = this.cmp(key, entries[mid][0]);
      if (order === 0) return this.#at(mid);
      if (order > 0) low = mid + 1;
      else high = mid;
    }
    return { there: false, i: low };
  }
  #at = (i) => {
    const entry = this.#entries[i];
    return { there: true, key: entry[0], value: entry[1], entry, i };
  };
  // The least entry's, or the greatest's (`last`), as `found` gives one,
  // asking nothing; `undefined` where it's empty.
  endFound(last) {
    const length = this.#entries.length;
    return length === 0 ? undefined : this.#at(last ? length - 1 : 0);
  }
  // What a search found given `value`, or `key` and `value` put where it
  // would go: the entry written. Or taken out, or the key it holds given
  // `key` in its place, a set's `replace`: no search again.
  putFound(found, key, value) {
    if (found.there) {
      found.entry[1] = value;
      return found.entry;
    }
    const entry = [key, value];
    this.#entries.splice(found.i, 0, entry);
    return entry;
  }
  deleteFound(found) {
    if (found.there) this.#entries.splice(found.i, 1);
  }
  replaceFound(found, key) {
    found.entry[0] = key;
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
    const of = new Map(Array.from(this.held(), (entry) => [entry[0], entry]));
    return Array.from(entries, ([key]) => [key, $entryHandle(of.get(key))]);
  }
  // Those from `key` on, taken out, in one of its own: one search.
  splitOff(key) {
    const rest = this.made();
    rest.#entries = this.#entries.splice(this.found(key).i);
    return rest;
  }
  // An empty one of its kind: a set's own map is one of its `cmp`, a map's
  // of its class.
  made() {
    return this.constructor === $OrdMap ? new $OrdMap(this.cmp) : new this.constructor();
  }
  // Its entries as it holds them, in order.
  held() {
    return this.#entries;
  }
  get size() {
    return this.#entries.length;
  }
  clear() {
    this.#entries = [];
  }
  get(key) {
    return this.found(key).value;
  }
  set(key, value) {
    this.putFound(this.found(key), key, value);
    return this;
  }
  has(key) {
    return this.found(key).there;
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
  *entries() {
    for (const [key, value] of this.held()) {
      yield [key, value];
    }
  }
  *keys() {
    for (const [key] of this.held()) {
      yield key;
    }
  }
  *values() {
    for (const [, value] of this.held()) {
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

// A handle on an entry's value, `$mutGet`'s kind.
function $entryHandle(entry) {
  return {
    get value() {
      return entry[1];
    },
    set value(item) {
      entry[1] = item;
    },
  };
}

// A set of items it finds by their own `Ord`: an `$OrdMap` of each to
// itself, or an `$OrdTree` (`Items`), each of its calls one search.
class $OrdSet extends Set {
  #items;
  constructor(cmp, items, Items = $OrdMap) {
    super();
    this.#items = new Items(cmp);
    if (items) {
      for (const item of items) {
        this.add(item);
      }
    }
  }
  static ofSorted(items, from) {
    const set = new this();
    const shape = from !== undefined && #items in from ? from.#items : undefined;
    set.#items.inOrder(
      Array.from(items, (item) => [item, item]),
      shape,
    );
    return set;
  }
  get size() {
    return this.#items.size;
  }
  found(item) {
    return this.#items.found(item);
  }
  endFound(last) {
    return this.#items.endFound(last);
  }
  putFound(found, item) {
    // One equal to it held stays: only its value, unread, is written.
    return this.#items.putFound(found, item, item);
  }
  deleteFound(found) {
    this.#items.deleteFound(found);
  }
  replaceFound(found, item) {
    this.#items.replaceFound(found, item);
  }
  deleteHeld(item) {
    this.#items.deleteHeld(item);
  }
  splitOff(item) {
    const rest = new this.constructor();
    rest.#items = this.#items.splitOff(item);
    return rest;
  }
  // Its own map, which a tree's operations are of.
  get items() {
    return this.#items;
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
    static way = { sorted: "own", cmp };
    constructor(entries) {
      super(cmp, entries);
    }
  };
}

function $ordSet(cmp) {
  return class extends $OrdSet {
    static way = { sorted: "own", cmp };
    constructor(items) {
      super(cmp, items);
    }
  };
}
