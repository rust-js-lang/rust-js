
// A flattened field (serde's `FlatMapDeserializer`): what's left of its
// struct's object. A struct takes the entries it has fields for, a map or an
// untagged or internally tagged enum sees every one left, and an enum takes
// the first that names a variant. Anything else can't be flattened.
class $JsonFlat extends $JsonDecoder {
  constructor(entries) {
    super();
    this.entries = entries;
  }

  other() {
    return $jsonError("can only flatten structs and maps");
  }

  deserializeNumber() {
    throw this.other();
  }

  deserializeStr() {
    throw this.other();
  }

  deserializeSeq() {
    throw this.other();
  }

  bool() {
    throw this.other();
  }

  identifier() {
    throw this.other();
  }

  deserializeMap(expected, visitMap) {
    return visitMap(new $JsonFlatMap(this.entries));
  }

  deserializeStruct(expected, names, visitMap) {
    return visitMap(new $JsonFlatStruct(this.entries, names));
  }

  deserializeAny(expected, visitor) {
    return this.deserializeMap(expected, (map) => {
      if (!visitor.map) throw $jsonError(`invalid type: map, expected ${expected}`);
      return visitor.map(map);
    });
  }

  unit() {
    return undefined;
  }

  unitStruct() {
    return undefined;
  }

  ignoreValue() {}

  // An `Option`: `None` when what it holds can't be read from what's left.
  option(read) {
    try {
      return read(this);
    } catch (e) {
      if (!(e instanceof $JsonError)) throw e;
      return undefined;
    }
  }

  content() {
    return { type: "map", value: this.entries.filter(Boolean) };
  }

  enum(enumName, variants, visit, other) {
    const names = variants.flatMap((names) => names);
    const i = this.entries.findIndex((entry) => entry && entry[0].type === "str" && names.includes(entry[0].value));
    if (i < 0) throw $jsonError(`no variant of enum ${enumName} found in flattened data`);
    const entry = this.entries[i];
    this.entries[i] = null;
    return new $JsonContent({ type: "map", value: [entry] }, true).enum(enumName, variants, visit, other);
  }
}

// `FlatStructAccess`: the entries a struct has fields for, taken.
class $JsonFlatStruct {
  constructor(entries, names) {
    this.entries = entries;
    this.names = names;
    this.index = 0;
  }

  next() {
    while (this.index < this.entries.length) {
      const i = this.index++;
      const entry = this.entries[i];
      if (entry && entry[0].type === "str" && this.names.includes(entry[0].value)) {
        this.entries[i] = null;
        this.entry = entry;
        return true;
      }
    }
    return false;
  }

  key() {
    return new $JsonContentKey(this.entry[0], true);
  }

  value(read) {
    return read(new $JsonContent(this.entry[1], true));
  }
}

// `FlatMapAccess`: every entry left, seen but left for the fields after.
class $JsonFlatMap {
  constructor(entries) {
    this.entries = entries;
    this.index = 0;
  }

  next() {
    while (this.index < this.entries.length) {
      const entry = this.entries[this.index++];
      if (entry) {
        this.entry = entry;
        return true;
      }
    }
    return false;
  }

  key() {
    return new $JsonContentKey(this.entry[0], false);
  }

  value(read) {
    return read(new $JsonContent(this.entry[1], false));
  }
}
