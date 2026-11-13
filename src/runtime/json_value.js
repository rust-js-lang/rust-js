
// serde_json's `Value` (ADR 0083): `"Null"`, or `{ TAG, _0 }` of `Bool`,
// `Number`, `String`, `Array` or `Object`, whose `Map` is a JS `Map`. A
// `Number` is `{ kind, value }`: `"u"` and `"i"` are integers (a BigInt past
// 2^53), `"f"` a finite float.

// `Number::from` of an integer, and `Value::from` of an `f64`, which is
// `Null` unless it's finite.
function $jsonInt(n) {
  return { kind: n < 0 ? "i" : "u", value: n };
}
// A 128-bit one's, serde_json's `serialize_i128` and `serialize_u128` of its
// value serializer: an error unless it fits in an `i64` or a `u64` (#1547).
function $jsonWideInt(n) {
  if (n < -9223372036854775808n || n > 18446744073709551615n) throw $jsonError("number out of range");
  return $jsonInt(n);
}
function $jsonFloat(x) {
  return Number.isFinite(x) ? { TAG: "Number", _0: { kind: "f", value: x } } : "Null";
}
// `Number::from_f64`: `None` unless it's finite.
function $jsonNumberOfF64(x) {
  return Number.isFinite(x) ? { kind: "f", value: x } : undefined;
}
// `Value::from` of an `Option`: `Null`, or what it holds, converted.
function $jsonOption(o, convert) {
  return o == null ? "Null" : convert(o);
}

function $jsonNumberWrite(n, json) {
  if (n.kind === "f") json.number(n.value);
  else json.int(n.value);
}

// serde_json's `Serialize` for `Value`: an object's keys in the order its
// `BTreeMap` keeps them.
function $jsonValueWrite(value, json) {
  if (value === "Null") return json.null();
  const { TAG: tag, _0: inner } = value;
  if (tag === "Bool") return json.bool(inner);
  if (tag === "Number") return $jsonNumberWrite(inner, json);
  if (tag === "String") return json.string(inner);
  if (tag === "Array") {
    json.beginArray();
    for (const item of inner) {
      json.element();
      $jsonValueWrite(item, json);
    }
    return json.endArray();
  }
  json.beginObject();
  for (const [key, item] of $sortedEntries(inner, $cmp)) {
    json.key(key);
    $jsonValueWrite(item, json);
  }
  json.endObject();
}

// `clone()`: a copy of each array and map, which change in place; a
// string or a number can't.
function $jsonValueClone(value) {
  if (value.TAG === "Array") return { TAG: "Array", _0: value._0.map($jsonValueClone) };
  if (value.TAG === "Object") return { TAG: "Object", _0: $jsonMapClone(value._0) };
  return value;
}
function $jsonMapClone(map) {
  return new Map(Array.from(map, ([key, item]) => [key, $jsonValueClone(item)]));
}

// `{}` of a `Value`, its JSON, and with `{:#}`, its pretty JSON.
function $jsonValueText(value, pretty) {
  return $toJson(value, $jsonValueWrite, pretty)._0;
}
function $jsonNumberText(n) {
  return n.kind === "f" ? $jsonNumber(n.value) : String(n.value);
}

// serde_json's `Debug`: `Null`, `Bool(true)`, `Number(1)`, `String("a")`,
// `Array [..]` and `Object {..}`, whose parts `{:#?}` puts on lines of their
// own, as a `Vec`'s and a map's.
function $debugJsonValue(value, alternate = false) {
  if (value === "Null") return "Null";
  const { TAG: tag, _0: inner } = value;
  if (tag === "Bool") return `Bool(${inner})`;
  if (tag === "Number") return $debugJsonNumber(inner);
  if (tag === "String") return `String(${$debugStr(inner)})`;
  if (tag === "Array") {
    const items = inner.map((item) => $debugJsonValue(item, alternate));
    return alternate ? "Array " + $pretty("[", items, "]") : `Array [${items.join(", ")}]`;
  }
  const entries = $sortedEntries(inner, $cmp).map(
    ([key, item]) => `${$debugStr(key)}: ${$debugJsonValue(item, alternate)}`,
  );
  return alternate ? "Object " + $pretty("{", entries, "}") : `Object {${entries.join(", ")}}`;
}
function $debugJsonNumber(n) {
  return `Number(${$jsonNumberText(n)})`;
}

// `as_bool`, `as_str`, `as_array`, `as_object`: what the variant `tag`
// holds, if it's that one.
function $jsonValueAs(value, tag) {
  return value.TAG === tag ? value._0 : undefined;
}
function $jsonValueF64(value) {
  return value.TAG === "Number" ? $jsonNumberF64(value._0) : undefined;
}
function $jsonNumberF64(n) {
  return Number(n.value);
}
// `as_u64()` and `as_i64()`: the integer, a BigInt, if it's of that kind
// and fits.
function $jsonNumberInt(n, kind) {
  const fits = kind === "u64" ? n.kind === "u" : $jsonNumberIsI64(n);
  return fits ? BigInt(n.value) : undefined;
}
function $jsonValueInt(value, kind) {
  return value.TAG === "Number" ? $jsonNumberInt(value._0, kind) : undefined;
}
function $jsonNumberIsI64(n) {
  return n.kind === "i" || (n.kind === "u" && n.value <= 9223372036854775807n);
}
// `is_f64`, `is_u64` and `is_i64` of a `Value`.
function $jsonValueIs(value, kind) {
  if (value.TAG !== "Number") return false;
  if (kind === "i64") return $jsonNumberIsI64(value._0);
  return value._0.kind === kind[0];
}

// `get(key)` of an object, or `get(i)` of an array, and `value[key]`, which is
// `Null` for what isn't there.
function $jsonGet(value, key) {
  if (typeof key === "string") return value.TAG === "Object" ? value._0.get(key) : undefined;
  return value.TAG === "Array" ? value._0[key] : undefined;
}
// `get_mut(key)` of a `&mut Value`, a cell of it, `null`'s string being no
// object (ADR 0083): a handle on the entry, which reads and writes it in its
// object's map or its array, or `undefined` if it isn't there.
function $jsonGetMut(cell, key) {
  const value = cell.value;
  const items = typeof key === "string" ? value.TAG === "Object" && value._0 : value.TAG === "Array" && value._0;
  if (!items || (typeof key === "string" ? !items.has(key) : key >= items.length)) return undefined;
  return typeof key === "string"
    ? { get value() { return items.get(key); }, set value(next) { items.set(key, next); } }
    : { get value() { return items[key]; }, set value(next) { items[key] = next; } };
}
// `value[key]` written, `IndexMut` of a `&mut Value`, a cell of it: a
// handle on the entry, as serde_json's `index_or_insert` (#1624). Of a key,
// a `null` made an empty object first, then its entry, `null` inserted
// where it isn't there; of an index, the array's item. Else serde_json's
// panic, naming what the value is.
function $jsonIndexMut(cell, key) {
  const kinds = { Bool: "boolean", Number: "number", String: "string", Array: "array", Object: "object" };
  const kind = (value) => (value === "Null" ? "null" : kinds[value.TAG]);
  if (typeof key === "string") {
    if (cell.value === "Null") cell.value = { TAG: "Object", _0: new Map() };
    const value = cell.value;
    if (value.TAG !== "Object") throw new Error(`cannot access key ${$debugStr(key)} in JSON ${kind(value)}`);
    const entries = value._0;
    if (!entries.has(key)) entries.set(key, "Null");
    return { get value() { return entries.get(key); }, set value(next) { entries.set(key, next); } };
  }
  const value = cell.value;
  if (value.TAG !== "Array") throw new Error(`cannot access index ${key} of JSON ${kind(value)}`);
  const items = value._0;
  if (key >= items.length) throw new Error(`cannot access index ${key} of JSON array of length ${items.length}`);
  return { get value() { return items[key]; }, set value(next) { items[key] = next; } };
}
// A JSON Pointer's tokens, as serde_json's `pointer` splits it: none of `""`,
// which names the value itself; `undefined` of one that doesn't start with
// `/`; else each after a `/`, its `~1` a `/`, then its `~0` a `~` (#1624).
function $jsonPointerTokens(pointer) {
  if (pointer === "") return [];
  if (!pointer.startsWith("/")) return undefined;
  return pointer.split("/").slice(1).map((token) => token.replaceAll("~1", "/").replaceAll("~0", "~"));
}
// Where `token` leads in `value`: an object's key it has, or an array's
// index in it, as serde_json's `parse_index` reads one, digits alone, no `+`
// and no leading `0`; else `undefined`.
function $jsonPointerStep(value, token) {
  if (value.TAG === "Object") return value._0.has(token) ? token : undefined;
  if (value.TAG !== "Array" || !/^(0|[1-9][0-9]*)$/.test(token)) return undefined;
  const index = Number(token);
  return index < value._0.length ? index : undefined;
}
// `pointer(p)`: the value a JSON Pointer names, or `undefined`.
function $jsonPointer(value, pointer) {
  const tokens = $jsonPointerTokens(pointer);
  if (tokens === undefined) return undefined;
  let target = value;
  for (const token of tokens) {
    const at = $jsonPointerStep(target, token);
    if (at === undefined) return undefined;
    target = typeof at === "string" ? target._0.get(at) : target._0[at];
  }
  return target;
}
// `pointer_mut(p)` of a `&mut Value`, a cell of it: a handle on what the
// pointer names, `$jsonGetMut`'s at each step, or `undefined`.
function $jsonPointerMut(cell, pointer) {
  const tokens = $jsonPointerTokens(pointer);
  if (tokens === undefined) return undefined;
  let target = cell;
  for (const token of tokens) {
    const at = $jsonPointerStep(target.value, token);
    if (at === undefined) return undefined;
    target = $jsonGetMut(target, at);
  }
  return target;
}
// `take()` of a `&mut Value`, a cell of it: what it held, `null` left in its
// place, as `mem::replace(self, Value::Null)` (#1624).
function $jsonTake(cell) {
  const value = cell.value;
  cell.value = "Null";
  return value;
}
function $jsonIndex(value, key) {
  return $jsonGet(value, key) ?? "Null";
}

// serde_json's `PartialEq` of a `Value` and a `String`, a `bool` or a
// number: `as_str()`, `as_bool()`, `as_f64()`, `as_i64()` or `as_u64()`
// is it.
function $jsonValueEq(value, other, kind) {
  if (kind === "String" || kind === "Bool") return value.TAG === kind && value._0 === other;
  if (value.TAG !== "Number") return false;
  const n = value._0;
  if (kind === "f64") return Number(n.value) === other;
  if (kind === "f32") return (n.kind === "f" ? Math.fround(n.value) : $bigToF32(BigInt(n.value))) === other;
  if (kind === "i64") return $jsonNumberIsI64(n) && n.value == other;
  return n.kind === "u" && n.value == other;
}

// `serde_json::to_value`: a `Value` of what `write` writes, serde_json's
// value serializer. A `NaN` is `Null`, and a map's keys are strings.
function $toJsonValue(value, write) {
  const stack = [];
  let root;
  const put = (v) => {
    const top = stack[stack.length - 1];
    if (!top) root = v;
    else if (top.items) top.items.push(v);
    else top.entries.set(top.key, v);
  };
  const string = (s) => put({ TAG: "String", _0: s });
  const open = () => stack.push({ items: [] });
  const builder = {
    null: () => put("Null"),
    bool: (b) => put({ TAG: "Bool", _0: b }),
    int: (n, wide) => put({ TAG: "Number", _0: wide === undefined ? $jsonInt(n) : $jsonWideInt(n) }),
    number: (x) => put($jsonFloat(x)),
    char: string,
    string,
    variant: string,
    beginArray: open,
    beginTuple: open,
    beginTupleStruct: open,
    element() {},
    endArray: () => put({ TAG: "Array", _0: stack.pop().items }),
    beginObject: () => stack.push({ entries: new Map() }),
    key(k) {
      stack[stack.length - 1].key = k;
    },
    endObject: () => put({ TAG: "Object", _0: stack.pop().entries }),
    flat(v, writeFlat, within) {
      writeFlat(v, $jsonFlat(builder, within));
    },
    // Where a protocol call began, and back to it where it failed: what
    // it opened dropped, its key with it (#1545).
    mark: () => stack.length,
    rewind(depth) {
      stack.length = depth;
      if (depth > 0) delete stack[depth - 1].key;
    },
  };
  try {
    write(value, builder);
  } catch (e) {
    if (e instanceof $JsonError) return { TAG: "Err", _0: { message: e.message, line: e.line, column: e.column } };
    throw e;
  }
  return { TAG: "Ok", _0: root };
}
