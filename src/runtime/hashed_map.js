// A hash map's or a hash set's keys found by their own `Hash` and `Eq` (ADR
// 1120), as std's table finds them: a search asks its `Hash`'s `hash`
// once, of a `$KeyHasher`, as of a std hasher, then `eq(key, held)` of each
// key held under the bytes it wrote, in the order they were put; keys whose bytes differ are never
// compared. Each entry is `[key, value, bytes]`, the one array wherever
// it's kept, so a handle on it holds (`$entry`), kept in `table` under the
// key it holds.
class $HashedKeys {
  #table = new Map();
  #under = new Map();
  constructor(KHash, eq) {
    this.KHash = KHash;
    this.eq = eq;
  }
  get size() {
    return this.#table.size;
  }
  // One search's answer: whether `key` is there, the key it holds equal to
  // it, its value and its entry; or where it would go, its bytes.
  found(key) {
    const hasher = new $KeyHasher();
    this.KHash.hash(key, { value: hasher }, $sipHasher);
    const bytes = hasher.written;
    const entry = this.#under.get(bytes)?.find((held) => this.eq(key, held[0]));
    if (entry === undefined) return { there: false, bytes };
    return { there: true, key: entry[0], value: entry[1], entry };
  }
  // What a search found given `value`, or `key` and `value` put where it
  // would go: the entry written, no search again.
  put(found, key, value) {
    if (found.there) {
      found.entry[1] = value;
      return found.entry;
    }
    const entry = [key, value, found.bytes];
    this.#table.set(key, entry);
    const under = this.#under.get(found.bytes);
    if (under === undefined) this.#under.set(found.bytes, [entry]);
    else under.push(entry);
    return entry;
  }
  // The entry of `key`, one it holds, taken out by what it is, asking
  // nothing, as std's `retain` takes out what it walks to.
  deleteHeld(key) {
    const entry = this.#table.get(key);
    if (entry === undefined) return;
    this.#table.delete(key);
    const under = this.#under.get(entry[2]);
    under.splice(under.indexOf(entry), 1);
    if (under.length === 0) this.#under.delete(entry[2]);
  }
  // The key a search found given `key` in its place, a set's `replace`.
  replace(found, key) {
    this.#table.delete(found.key);
    found.entry[0] = key;
    this.#table.set(key, found.entry);
  }
  clear() {
    this.#table.clear();
    this.#under.clear();
  }
  entries() {
    return this.#table.values();
  }
}

// The class of a hash map of keys found by their `Hash`, `KHash`, and `eq`:
// a `Map`, which its methods find through, one search each.
function $hashedMap(KHash, eq) {
  return class extends Map {
    static way = { find: "hashed", KHash, eq };
    #keys = new $HashedKeys(KHash, eq);
    constructor(entries) {
      super();
      if (entries) {
        for (const [key, value] of entries) this.set(key, value);
      }
    }
    get size() {
      return this.#keys.size;
    }
    found(key) {
      return this.#keys.found(key);
    }
    putFound(found, key, value) {
      return this.#keys.put(found, key, value);
    }
    deleteFound(found) {
      if (found.there) this.#keys.deleteHeld(found.key);
    }
    deleteHeld(key) {
      this.#keys.deleteHeld(key);
    }
    // The key it holds equal to `key`, as a map keeps it (`$keptAs`).
    keyOf(key) {
      const found = this.found(key);
      return found.there ? found.key : key;
    }
    storedKey(key) {
      return this.found(key).key;
    }
    get(key) {
      return this.found(key).value;
    }
    has(key) {
      return this.found(key).there;
    }
    set(key, value) {
      this.putFound(this.found(key), key, value);
      return this;
    }
    delete(key) {
      const found = this.found(key);
      this.deleteFound(found);
      return found.there;
    }
    clear() {
      this.#keys.clear();
    }
    *entries() {
      for (const [key, value] of this.#keys.entries()) yield [key, value];
    }
    *keys() {
      for (const [key] of this.#keys.entries()) yield key;
    }
    *values() {
      for (const [, value] of this.#keys.entries()) yield value;
    }
    [Symbol.iterator]() {
      return this.entries();
    }
    forEach(f, that) {
      for (const [key, value] of this.entries()) f.call(that, value, key, this);
    }
  };
}

// The class of a hash set of items found by `hash` and `eq`: a `Set`, as
// `$hashedMap`'s.
function $hashedSet(KHash, eq) {
  return class extends Set {
    static way = { find: "hashed", KHash, eq };
    #keys = new $HashedKeys(KHash, eq);
    constructor(items) {
      super();
      if (items) {
        for (const item of items) this.add(item);
      }
    }
    get size() {
      return this.#keys.size;
    }
    found(item) {
      return this.#keys.found(item);
    }
    putFound(found, item) {
      return this.#keys.put(found, item);
    }
    deleteFound(found) {
      if (found.there) this.#keys.deleteHeld(found.key);
    }
    replaceFound(found, item) {
      this.#keys.replace(found, item);
    }
    deleteHeld(item) {
      this.#keys.deleteHeld(item);
    }
    keyOf(item) {
      const found = this.found(item);
      return found.there ? found.key : item;
    }
    storedKey(item) {
      return this.found(item).key;
    }
    has(item) {
      return this.found(item).there;
    }
    add(item) {
      const found = this.found(item);
      if (!found.there) this.putFound(found, item);
      return this;
    }
    delete(item) {
      const found = this.found(item);
      this.deleteFound(found);
      return found.there;
    }
    clear() {
      this.#keys.clear();
    }
    *values() {
      for (const [item] of this.#keys.entries()) yield item;
    }
    keys() {
      return this.values();
    }
    *entries() {
      for (const [item] of this.#keys.entries()) yield [item, item];
    }
    [Symbol.iterator]() {
      return this.values();
    }
    forEach(f, that) {
      for (const item of this.values()) f.call(that, item, item, this);
    }
  };
}
