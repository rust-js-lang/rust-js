
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
    if (this.data.type === "str") return visit(this.data.value, this.data.borrowed);
    throw this.invalid(expected);
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
    if (visit) return visit(value);
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

  value(read) {
    return read(this.reader);
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

  // `has_next_key`.
  next() {
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
    return new $JsonKey(this.reader);
  }

  value(read) {
    this.reader.parseObjectColon();
    return read(this.reader);
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
// is read from inside the quotes.
class $JsonKey {
  constructor(reader) {
    this.reader = reader;
  }

  string() {
    this.reader.index++;
    return this.reader.parseStr();
  }

  char() {
    return $jsonChar(this.string());
  }

  ignore() {
    this.string();
  }

  content() {
    const value = this.string();
    return { type: "str", value, borrowed: !this.reader.escaped };
  }

  number(read) {
    const reader = this.reader;
    reader.index++;
    const c = reader.peek();
    if (!(c === 45 || (c >= 48 && c <= 57))) throw reader.error("invalid value: expected key to be a number in quotes");
    const value = read(reader);
    if (reader.peek() !== 34) throw reader.peekError('expected `"`');
    reader.index++;
    return value;
  }

  bool() {
    const reader = this.reader;
    reader.index++;
    try {
      const c = reader.next();
      if (c === -1) throw reader.peekError("EOF while parsing a value");
      if (c === 116) {
        reader.parseIdent('rue"');
        return true;
      }
      if (c === 102) {
        reader.parseIdent('alse"');
        return false;
      }
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
