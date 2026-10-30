
// A `serde_json::Value` read as a deserializer (ADR 0083), for
// `serde_json::from_value`: its `impl Deserializer for Value`.
class $JsonValueReader extends $JsonDecoder {
  constructor(value) {
    super();
    this.value = value;
  }

  // serde_json's `Value::unexpected`.
  unexpected() {
    const value = this.value;
    if (value === "Null") return "null";
    const { TAG: tag, _0: inner } = value;
    if (tag === "Bool") return `boolean \`${inner}\``;
    if (tag === "Number") return $jsonUnexpectedNumber(inner);
    if (tag === "String") return `string ${$debugStr(inner)}`;
    return tag === "Array" ? "sequence" : "map";
  }

  invalid(expected) {
    return $jsonError(`invalid type: ${this.unexpected()}, expected ${expected}`);
  }

  deserializeNumber(expected, visit) {
    if (this.value.TAG === "Number") return visit(this.value._0);
    throw this.invalid(expected);
  }

  // A string of a `Value` is its own, so a `&str` can't borrow it.
  deserializeStr(expected, visit) {
    if (this.value.TAG === "String") return visit(this.value._0, false);
    throw this.invalid(expected);
  }

  visitArray(visit) {
    const items = this.value._0;
    const seq = new $JsonValueSeq(items);
    const value = visit(seq);
    if (seq.count < items.length) throw $jsonError(`invalid length ${items.length}, expected fewer elements in array`);
    return value;
  }

  visitObject(visit) {
    const entries = $sortedEntries(this.value._0, $cmp);
    const map = new $JsonValueMap(entries);
    const value = visit(map);
    if (map.count < entries.length) throw $jsonError(`invalid length ${entries.length}, expected fewer elements in map`);
    return value;
  }

  deserializeSeq(expected, visit) {
    if (this.value.TAG === "Array") return this.visitArray(visit);
    throw this.invalid(expected);
  }

  deserializeMap(expected, visitMap, visitSeq) {
    if (this.value.TAG === "Array" && visitSeq) return this.visitArray(visitSeq);
    if (this.value.TAG === "Object") return this.visitObject(visitMap);
    throw this.invalid(expected);
  }

  deserializeAny(expected, visitor) {
    const value = this.value;
    if (value === "Null") {
      if (visitor.unit) return visitor.unit();
      throw this.invalid(expected);
    }
    const { TAG: tag, _0: inner } = value;
    if (tag === "Array") {
      return this.visitArray((seq) => {
        if (!visitor.seq) throw this.invalid(expected);
        return visitor.seq(seq);
      });
    }
    if (tag === "Object") {
      return this.visitObject((map) => {
        if (!visitor.map) throw this.invalid(expected);
        return visitor.map(map);
      });
    }
    const visit = { Bool: visitor.bool, Number: visitor.number, String: visitor.string }[tag];
    if (visit) return visit(inner, false);
    throw this.invalid(expected);
  }

  bool() {
    if (this.value.TAG === "Bool") return this.value._0;
    throw this.invalid("a boolean");
  }

  unit(expected = "unit") {
    if (this.value === "Null") return undefined;
    throw this.invalid(expected);
  }

  unitStruct(expected) {
    return this.unit(expected);
  }

  option(read) {
    return this.value === "Null" ? undefined : read(this);
  }

  ignoreValue() {}

  // serde's `ContentVisitor` of it: a string is an owned one.
  content() {
    return $jsonValueContent(this.value);
  }

  enum(enumName, variants, visit, other) {
    const value = this.value;
    let name;
    let held;
    if (value.TAG === "Object") {
      if (value._0.size !== 1) throw $jsonError("invalid value: map, expected map with a single key");
      [[name, held]] = value._0;
    } else if (value.TAG === "String") {
      name = value._0;
    } else {
      throw this.invalid("string or map");
    }
    return visit($jsonVariantNamed(name, variants, other), new $JsonValueVariant(held));
  }

  identifier(variants, other) {
    return this.deserializeStr("variant identifier", (name) => $jsonVariantNamed(name, variants, other));
  }
}

function $jsonValueContent(value) {
  if (value === "Null") return { type: "unit" };
  const { TAG: tag, _0: inner } = value;
  if (tag === "Bool") return { type: "bool", value: inner };
  if (tag === "Number") return { type: "num", value: inner };
  if (tag === "String") return { type: "str", value: inner, borrowed: false };
  if (tag === "Array") return { type: "seq", value: inner.map($jsonValueContent) };
  const entries = $sortedEntries(inner, $cmp).map(([key, item]) => [
    { type: "str", value: key, borrowed: false },
    $jsonValueContent(item),
  ]);
  return { type: "map", value: entries };
}

class $JsonValueSeq {
  constructor(items) {
    this.items = items;
    this.count = 0;
  }

  next() {
    return this.count < this.items.length && ++this.count > 0;
  }

  value(read) {
    return read(new $JsonValueReader(this.items[this.count - 1]));
  }

  element(read, i, expected, missing, defaults) {
    return $JsonSeq.prototype.element.call(this, read, i, expected, missing, defaults);
  }
}

class $JsonValueMap {
  constructor(entries) {
    this.entries = entries;
    this.count = 0;
  }

  next() {
    return this.count < this.entries.length && ++this.count > 0;
  }

  key() {
    return new $JsonValueKey(this.entries[this.count - 1][0]);
  }

  value(read) {
    return read(new $JsonValueReader(this.entries[this.count - 1][1]));
  }
}

// serde_json's `MapKeyDeserializer`: an object's key, a number read from it
// as JSON text would be.
class $JsonValueKey {
  constructor(key) {
    this.key = key;
  }

  string() {
    return this.key;
  }

  char() {
    return $jsonChar(this.key);
  }

  ignore() {}

  content() {
    return { type: "str", value: this.key, borrowed: false };
  }

  number(read) {
    const json = new $JsonReader(this.key);
    const c = json.peek();
    if (!(c === 45 || (c >= 48 && c <= 57))) throw $jsonError("invalid value: expected key to be a number in quotes");
    const value = read(json);
    if (json.peek() !== -1) throw $jsonError("invalid value: expected key to be a number in quotes");
    return value;
  }

  bool() {
    if (this.key === "true" || this.key === "false") return this.key === "true";
    throw $jsonError(`invalid type: string ${$debugStr(this.key)}, expected a boolean`);
  }
}

// `VariantAccess` of a `Value`'s variant: what it holds, if anything.
class $JsonValueVariant {
  constructor(held) {
    this.held = held;
  }

  reader(kind, tag) {
    if (this.held === undefined) throw $jsonError(`invalid type: unit variant, expected ${kind}`);
    const reader = new $JsonValueReader(this.held);
    if (tag && this.held.TAG !== tag) throw reader.invalid(kind);
    return reader;
  }

  unit() {
    if (this.held !== undefined) new $JsonValueReader(this.held).unit();
  }

  newtype(read) {
    return read(this.reader("newtype variant"));
  }

  tuple(expected, fields, build, options) {
    const reader = this.reader("tuple variant", "Array");
    // An empty array is a unit, which a tuple variant isn't.
    if (this.held._0.length === 0) throw $jsonError(`invalid type: null, expected ${options?.expecting ?? expected}`);
    return reader.tupleStruct(expected, fields, build, options);
  }

  struct(expected, fields, build, options) {
    if (options?.flatten) return this.reader("newtype variant").struct(expected, fields, build, options);
    return this.reader("struct variant", "Object").struct(expected, fields, build, options);
  }
}

// `serde_json::from_value`: a `Result` of what `read` reads of `value`.
function $fromJsonValue(value, read) {
  try {
    return { TAG: "Ok", _0: read(new $JsonValueReader(value)) };
  } catch (e) {
    if (e instanceof $JsonError) return { TAG: "Err", _0: { message: e.message, line: e.line, column: e.column } };
    throw e;
  }
}

// serde's `missing_field`: what a field that isn't there reads as, which is
// `None` for an `Option`, and otherwise an error. An adjacently tagged
// enum's content that isn't there is read so too, and is nothing for a
// unit variant. Made by a pure call, its methods set in it, so that a
// bundler drops it where nothing makes one (rd274).
const $JsonMissing = /* @__PURE__ */ (() => {
  class JsonMissing {
    constructor(name) {
      this.name = name;
    }

    option() {
      return undefined;
    }

    untaggedUnit() {
      return undefined;
    }
  }
  for (const method of [
    "bool", "int", "f64", "f32", "string", "char", "borrowedStr", "unit", "unitStruct", "vec", "tuple", "array", "map",
    "struct", "untaggedStruct", "tupleStruct", "enum", "taggedUnit", "internallyTagged",
    "adjacentlyTagged", "untagged",
  ]) {
    JsonMissing.prototype[method] = function () {
      throw $jsonError(`missing field \`${this.name}\``);
    };
  }
  return JsonMissing;
})();
