
// A value already read (serde's `ContentDeserializer`, or when not `owned`,
// its `ContentRefDeserializer`): `{ type, value }`, where `type` is `unit`,
// `bool`, `num`, `str`, `seq` or `map`.
class $JsonContent extends $JsonDecoder {
  constructor(content, owned) {
    super();
    this.data = content;
    this.owned = owned;
  }

  invalid(expected) {
    return $jsonError(`invalid type: ${$jsonUnexpectedContent(this.data)}, expected ${expected}`);
  }

  deserializeNumber(expected, visit) {
    if (this.data.type === "num") return visit(this.data.value);
    throw this.invalid(expected);
  }

  deserializeStr(expected, visit) {
    if (this.data.type === "str") return visit(this.data.value, this.stringKind());
    throw this.invalid(expected);
  }

  // The kind its string is given as (#1546): one borrowed from the text,
  // `visit_borrowed_str`; one it holds, given whole where it's owned,
  // serde's `ContentDeserializer`'s `visit_string`, and by reference
  // where it isn't, its `ContentRefDeserializer`'s `visit_str`.
  stringKind() {
    return this.data.borrowed ? "borrowed" : this.owned ? "owned" : "transient";
  }

  visitSeq(visit) {
    const seq = new $JsonContentSeq(this.data.value, this.owned);
    const value = visit(seq);
    seq.end();
    return value;
  }

  visitMap(visit) {
    const map = new $JsonContentMap(this.data.value, this.owned);
    const value = visit(map);
    map.end();
    return value;
  }

  deserializeSeq(expected, visit) {
    if (this.data.type === "seq") return this.visitSeq(visit);
    throw this.invalid(expected);
  }

  deserializeMap(expected, visitMap, visitSeq) {
    if (this.data.type === "seq" && visitSeq) return this.visitSeq(visitSeq);
    if (this.data.type === "map") return this.visitMap(visitMap);
    throw this.invalid(expected);
  }

  deserializeAny(expected, visitor) {
    const { type, value } = this.data;
    if (type === "seq" && visitor.seq) return this.visitSeq(visitor.seq);
    if (type === "map" && visitor.map) return this.visitMap(visitor.map);
    const visit = { unit: visitor.unit, bool: visitor.bool, num: visitor.number, str: visitor.string }[type];
    if (visit) return visit(value, type === "str" ? this.stringKind() : undefined);
    throw this.invalid(expected);
  }

  bool() {
    if (this.data.type === "bool") return this.data.value;
    throw this.invalid("a boolean");
  }

  unit(expected = "unit") {
    const { type, value } = this.data;
    // An owned one takes `{}` for a unit, as a newtype variant of `()`.
    if (type === "unit" || (this.owned && type === "map" && value.length === 0)) return undefined;
    throw this.invalid(expected);
  }

  unitStruct(expected) {
    const { type, value } = this.data;
    if (type === "unit" || (this.owned && (type === "map" || type === "seq") && value.length === 0)) return undefined;
    throw this.invalid(expected);
  }

  option(read) {
    return this.data.type === "unit" ? undefined : read(this);
  }

  // `deserialize_option` of a hand-written visitor, serde's
  // `ContentDeserializer`'s: a unit to its `visit_unit`, which a `null` is,
  // anything else to its `visit_some` (#1597).
  deserializeOption(expected, visitor) {
    if (this.data.type !== "unit") return visitor.some(this);
    if (visitor.unit) return visitor.unit();
    throw this.invalid(expected);
  }

  ignoreValue() {}

  content() {
    return this.data;
  }

  enum(enumName, variants, visit, other) {
    const { type, value } = this.data;
    let variant;
    let held;
    if (type === "map") {
      if (value.length !== 1) throw $jsonError("invalid value: map, expected map with a single key");
      [[variant, held]] = value;
    } else if (type === "str") {
      variant = this.data;
    } else {
      throw this.invalid("string or map");
    }
    const name = new $JsonContent(variant, this.owned).identifier(variants, other);
    return visit(name, new $JsonContentVariant(held, this.owned));
  }

  // A variant's name, or its index among the variants.
  identifier(variants, other) {
    const { type, value } = this.data;
    if (type === "str") return $jsonVariantNamed(value, variants, other);
    if (type !== "num" || value.kind !== "u") throw this.invalid("variant identifier");
    if (value.value < variants.length) return $jsonName(variants[Number(value.value)]);
    if (other !== undefined) return other;
    throw $jsonError(`invalid value: integer \`${value.value}\`, expected variant index 0 <= i < ${variants.length}`);
  }
}

// `SeqAccess`.
class $JsonSeq {
  constructor(reader) {
    this.reader = reader;
    this.first = true;
    this.index = 0;
  }

  // `has_next_element`.
  next() {
    const reader = this.reader;
    const peek = reader.parseWhitespace();
    if (peek === -1) throw reader.peekError("EOF while parsing a list");
    if (peek === 93) return false;
    if (this.first) {
      this.first = false;
      return true;
    }
    if (peek !== 44) throw reader.peekError("expected `,` or `]`");
    reader.index++;
    const c = reader.parseWhitespace();
    if (c === 93) throw reader.peekError("trailing comma");
    if (c === -1) throw reader.peekError("EOF while parsing a value");
    return true;
  }

  // Of a read serde_path_to_error tracks, the item is at its index:
  // `items[1]`, its comma's and its `]`'s mistakes the sequence's own.
  value(read) {
    const chain = this.reader.chain;
    if (!chain) return read(this.reader);
    chain.push(this.index++);
    try {
      return read(this.reader);
    } finally {
      chain.pop();
    }
  }

  // The `i`th of `expected`'s items.
  element(read, i, expected, missing, defaults) {
    if (this.next()) return this.value(read);
    if (missing) return missing(defaults);
    throw $jsonError(`invalid length ${i}, expected ${expected}`);
  }
}

// The items of a value already read.
class $JsonContentSeq {
  constructor(items, owned) {
    this.items = items;
    this.owned = owned;
    this.count = 0;
  }

  next() {
    return this.count < this.items.length && ++this.count > 0;
  }

  value(read) {
    return read(new $JsonContent(this.items[this.count - 1], this.owned));
  }

  element(read, i, expected, missing, defaults) {
    return $JsonSeq.prototype.element.call(this, read, i, expected, missing, defaults);
  }

  // Items left over: the length is wrong.
  end() {
    const length = this.items.length;
    if (this.count < length) {
      const count = this.count;
      throw $jsonError(`invalid length ${length}, expected ${count} element${count === 1 ? "" : "s"} in sequence`);
    }
  }
}

// `MapAccess`.
class $JsonMap {
  constructor(reader) {
    this.reader = reader;
    this.first = true;
  }

  // Of a read serde_path_to_error tracks, the next key is `?` until its
  // text is read, `a.?`, which a mistake finding it is at; its `}` the
  // object's own.
  next() {
    const chain = this.reader.chain;
    if (!chain) return this.hasNextKey();
    chain.push(null);
    const more = this.hasNextKey();
    if (!more) chain.pop();
    return more;
  }

  // `has_next_key`.
  hasNextKey() {
    const reader = this.reader;
    const peek = reader.parseWhitespace();
    if (peek === -1) throw reader.peekError("EOF while parsing an object");
    if (peek === 125) return false;
    if (this.first) {
      this.first = false;
      if (peek === 34) return true;
      throw reader.peekError("key must be a string");
    }
    if (peek !== 44) throw reader.peekError("expected `,` or `}`");
    reader.index++;
    const c = reader.parseWhitespace();
    if (c === 34) return true;
    throw reader.peekError(
      c === 125 ? "trailing comma" : c === -1 ? "EOF while parsing a value" : "key must be a string",
    );
  }

  key() {
    return new $JsonKey(this.reader, this);
  }

  // Of a read serde_path_to_error tracks, the value is at its key, `a.b`,
  // or `a.?` of one that wasn't text; its `:` the object's own.
  value(read) {
    this.reader.parseObjectColon();
    const chain = this.reader.chain;
    if (!chain) return read(this.reader);
    chain.push(this.keyRead);
    try {
      return read(this.reader);
    } finally {
      chain.pop();
    }
  }
}

// The entries of a value already read.
class $JsonContentMap {
  constructor(entries, owned) {
    this.entries = entries;
    this.owned = owned;
    this.count = 0;
  }

  next() {
    return this.count < this.entries.length && ++this.count > 0;
  }

  key() {
    return new $JsonContentKey(this.entries[this.count - 1][0], this.owned);
  }

  value(read) {
    return read(new $JsonContent(this.entries[this.count - 1][1], this.owned));
  }

  end() {
    const length = this.entries.length;
    if (this.count < length) {
      const count = this.count;
      throw $jsonError(`invalid length ${length}, expected ${count} element${count === 1 ? "" : "s"} in map`);
    }
  }
}

// `MapKey`: an object's key, from its opening quote. A number or a `bool`
// is read from inside the quotes. Of a read serde_path_to_error tracks, the
// key, once read, is where the rest of its reading is, its `CaptureKey`'s:
// its text, a number's digits, or a `bool`'s word; each method ends its
// reading, `read`, and the map keeps it for its value's place.
class $JsonKey {
  constructor(reader, map) {
    this.reader = reader;
    this.map = map;
  }

  text() {
    this.reader.index++;
    const key = this.reader.parseStr();
    this.reader.captureKey(key);
    return key;
  }

  read(value) {
    const chain = this.reader.chain;
    if (chain) this.map.keyRead = chain.pop();
    return value;
  }

  string() {
    return this.read(this.text());
  }

  char() {
    return this.read($jsonChar(this.text()));
  }

  ignore() {
    this.read(this.text());
  }

  content() {
    const value = this.text();
    return this.read({ type: "str", value, borrowed: !this.reader.escaped });
  }

  // A struct's field it names, as serde's derived identifier reads it:
  // an unknown one's mistake, `deny_unknown_fields`', at the key.
  field(fields, deny) {
    const key = this.text();
    const i = $jsonField(key, fields, deny);
    return this.read([{ type: "str", value: key, borrowed: !this.reader.escaped }, i]);
  }

  number(read) {
    const reader = this.reader;
    reader.index++;
    const c = reader.peek();
    if (!(c === 45 || (c >= 48 && c <= 57))) throw reader.error("invalid value: expected key to be a number in quotes");
    reader.keying = reader.chain !== undefined;
    const value = read(reader);
    reader.keying = false;
    if (reader.peek() !== 34) throw reader.peekError('expected `"`');
    reader.index++;
    return this.read(value);
  }

  bool() {
    const reader = this.reader;
    reader.index++;
    try {
      const c = reader.next();
      if (c === -1) throw reader.peekError("EOF while parsing a value");
      if (c === 116) {
        reader.parseIdent('rue"');
        reader.captureKey("true");
        return this.read(true);
      }
      if (c === 102) {
        reader.parseIdent('alse"');
        reader.captureKey("false");
        return this.read(false);
      }
      // Not visited: serde_json's own mistake, which isn't the key's.
      throw $jsonError(`invalid type: string ${$debugStr(reader.parseStr())}, expected a boolean`);
    } catch (e) {
      throw reader.fixPosition(e);
    }
  }
}

// A key already read: it's read as any other value is, so a number in
// quotes isn't a number, as in serde.
class $JsonContentKey {
  constructor(key, owned) {
    this.key = new $JsonContent(key, owned);
  }

  field(fields, deny) {
    const key = this.content();
    return [key, $jsonField(key.value, fields, deny)];
  }

  string() {
    return this.key.string();
  }

  char() {
    return this.key.char();
  }

  bool() {
    return this.key.bool();
  }

  ignore() {}

  content() {
    return this.key.content();
  }

  number(read) {
    return read(this.key);
  }
}

// `VariantAccess` of `{"Name": ..}`, or of `"Name"` (`unit`), which holds
// nothing.
class $JsonVariant {
  constructor(reader, unit) {
    this.reader = reader;
    this.isUnit = unit;
  }

  unit() {
    if (!this.isUnit) this.reader.unit();
  }

  newtype(read) {
    if (this.isUnit) throw $jsonError("invalid type: unit variant, expected newtype variant");
    return read(this.reader);
  }

  tuple(expected, fields, build, options) {
    if (this.isUnit) throw $jsonError("invalid type: unit variant, expected tuple variant");
    return this.reader.tupleStruct(expected, fields, build, options);
  }

  // With a flattened field, serde reads it as a newtype variant of a map.
  struct(expected, fields, build, options) {
    if (this.isUnit) {
      throw $jsonError(`invalid type: unit variant, expected ${options?.flatten ? "newtype" : "struct"} variant`);
    }
    return this.reader.struct(expected, fields, build, options);
  }
}

// The same, of a value already read: what a variant holds, if anything.
class $JsonContentVariant {
  constructor(held, owned) {
    this.held = held;
    this.owned = owned;
  }

  reader(kind, types) {
    if (this.held === undefined) throw $jsonError(`invalid type: unit variant, expected ${kind}`);
    if (types && !types.includes(this.held.type)) {
      throw $jsonError(`invalid type: ${$jsonUnexpectedContent(this.held)}, expected ${kind}`);
    }
    return new $JsonContent(this.held, this.owned);
  }

  unit() {
    if (this.held !== undefined) new $JsonContent(this.held, this.owned).unit();
  }

  newtype(read) {
    return read(this.reader("newtype variant"));
  }

  tuple(expected, fields, build, options) {
    return this.reader("tuple variant", ["seq"]).tupleStruct(expected, fields, build, options);
  }

  struct(expected, fields, build, options) {
    const reader = options?.flatten ? this.reader("newtype variant") : this.reader("struct variant", ["map", "seq"]);
    return reader.struct(expected, fields, build, options);
  }
}
