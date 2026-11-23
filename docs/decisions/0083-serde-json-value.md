# 0083. serde_json's `Value` is an enum like any other; its `Number` is `{ kind, value }` and its `Map` a `Map`

Status: Accepted. Extends [0033](0033-enums-with-fields.md), [0059](0059-hashmap.md) and [0077](0077-serde-json.md) to [0082](0082-serde-flatten.md).

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Not every JSON has a type: a field of settings, a response a client passes
along, a document built on the fly. Rust has serde_json's `Value` for it:

```rust
let v: Value = serde_json::from_str(text)?;
match &v["user"] {
    Value::Object(user) => user.get("name").and_then(|n| n.as_str()),
    _ => None,
}
json!({ "id": id, "tags": ["a", "b"] })
```

It was an error: a `Number` holds a `u64` or an `i64`, which rust-js can't
represent, and none of `Value`'s API was known.

## Decision

**A `Value` is an enum as rust-js makes one (ADR 0033):** `"Null"`, or `{
TAG: "Bool" | "Number" | "String" | "Array" | "Object", _0 }`. So a
`match` on one, `Value::String(s)`, and `json!`'s expansion are rust-js's
ordinary enums, with nothing of their own.

- **A `Number` is `{ kind, value }`,** `"u"` or `"i"` for an integer (a
  BigInt past 2^53), `"f"` for a float, as serde_json keeps one: `1` and
  `1.0` are different numbers, written as they were read.
- **A `Map<String, Value>` is a JS `Map`,** the `BTreeMap` it wraps (ADR
  0059), so its methods are a map's, and it's written in its keys' order.
- **Reading one is serde_json's `ValueVisitor`, and writing one its
  `Serialize`,** through the reader and writer of ADRs 0077 and 0078; a
  `Value` is read the same from text, from a kept value (ADR 0079), or
  flattened (ADR 0082).

**Its API is serde_json's, in small runtime functions:**

| Rust | JS |
|---|---|
| `v["k"]`, `v[0]` | `$jsonIndex(v, key)`: `"Null"` for what isn't there |
| `v.get("k")`, `v.get(0)` | `$jsonGet(v, key)`: `undefined` for what isn't there |
| `v.as_str()`, `as_bool`, `as_array`, `as_object` | `$jsonValueAs(v, "String")` |
| `v.is_null()`, `is_object`, .. | `v === "Null"`, `v.TAG === "Object"` |
| `v == "x"`, `v == 1`, `v == 1.0` | `$jsonValueEq(v, x, kind)`: `as_str()`, `as_i64()`, `as_f64()`, .. is it |
| `Value::from(x)`, `x.into()` | the variant of it; an `f64` that isn't finite is `Null` |
| `{}`, `{:#}`, `{:?}` | its JSON, pretty JSON, and serde_json's `Object {"a": Number(1)}` |
| `clone()`, `Value::default()` | a copy of its arrays and maps; `"Null"` |

**`to_value(&x)` is `x`'s serializer writing a `Value`** instead of text
(`$toJsonValue`), serde_json's value serializer: an object's keys are
strings, a unit variant is its name, and a `NaN` is `Null`. Of a string, a
number, a `bool` or `()`, which can't fail, it's the `Value` itself, and so
is `json!(1)`:

```js
object.set("y", { TAG: "Array", _0: [{ TAG: "Number", _0: $jsonInt(1) }, { TAG: "String", _0: "two" }, "Null"] });
```

**`from_value::<T>(v)` is serde_json's `impl Deserializer for Value`**
(`$JsonValueReader`): its errors have no place, a string is its own, so a
`&str` can't borrow it, an array with items left over is `invalid length 6,
expected fewer elements in array`, and a map's number keys are read as
JSON text is.

### Also here

- `Ok(x).unwrap()` of an `Ok` just made is `x`.

## Why

- **It's serde_json's answer.** The `dynamic` example reads, writes, matches,
  indexes, compares, clones, builds with `json!`, and converts with
  `to_value` and `from_value`, and every value, text and error matches
  native Rust.
- **Nothing new to learn in the JS:** a `Value` is an enum, a `Map` a
  `Map`, and a number says what kind it is.

## Alternatives

- **A `Value` as the JSON JS would parse:** `null`, `true`, `1`, `"x"`, an
  array, an object. Plain, but `1` and `1.0` would be one number, a big
  integer would lose digits, and a `match` would need its own lowering.

## Consequences

- `v["k"] = x` (`IndexMut`), which makes a `Null` an object, is an error:
  change it through `as_object_mut()`.
- `as_u64()` and `as_i64()` give 64-bit integers, which rust-js can't hold
  yet; `as_f64()` works. (They work now, as BigInts: ADR 0086.)

## Amendment: a `&mut Value` is a cell

A `Value`'s `Null` is the string `"Null"`, each other variant an object;
a `Value` was taken as an object all the same, written in place by
`$assign`, which a change to or from `Null` can't be: it threw (#169).
A `Value` isn't an object (`is_object`), so a `&mut Value` is a cell of
it, written whole; `get_mut(key)` is a handle on the entry,
`$jsonGetMut`, and `as_array_mut()` and `as_object_mut()` read what the
cell holds, whose array or map is changed in place. Held by the serde
test "a Value written through a &mut changes to and from Null".

## Amendment: written through `IndexMut`, and its access API (2026-11-13, #1624)

A `Value` written through `IndexMut`, `v["a"] = json!(1)`, `v[0] = ..` and
nested, is a handle on the entry, as `get_mut`'s is, of a `&mut Value`'s
cell (`$jsonIndexMut`), by serde_json's `index_or_insert`: of a key, a
`null` made an empty object first, then its entry, `null` inserted where
it isn't there; of an index, the array's item; else serde_json's panic,
`cannot access key "a" in JSON array`, `cannot access index 0 of JSON
object` or `cannot access index 2 of JSON array of length 2`. A map's
entries are sorted where they're written, so an inserted key needs no
place. And its access API: `take()`, `null` left in its place
(`$jsonTake`); `as_null()`, `Some(())` of a `null`; `pointer(p)` and
`pointer_mut(p)`, a JSON Pointer as serde_json reads one (`""` the value
itself, one not from `/` none, `~1` unescaped before `~0`, an array's
index digits alone, no `+` and no leading `0`), `pointer_mut`'s a handle
walked by `get_mut`'s steps. serde_json's `Map`'s `retain` is a sorted
map's, and its `map::Entry`, now one of serde_json's own types (`Json::
Entry`), is a `BTreeMap` entry's. Case (ADR 0262): none observable, the
same values, order and panics as serde_json's.

## Amendment: `IndexMut`'s panics named where serde_json raises them (2026-11-23, #2325)

`value[key] = ..` of what can't hold it panics where serde_json's
`index_or_insert` does, of the version Cargo built: a key of what's no
object, an index of what's no array, an index past an array's end, each its
own line of `src/value/index.rs`. serde_json is read as rustc builds it, so
its sites aren't walked of MIR, as std's are: Cargo's check of it through
rust-js finds each in that version's source by its `panic!`'s message
(`library::registry_sites`, the one finder and catalog, hashbrown's own
map's `get_disjoint_mut` too) and notes them beside its metadata,
`<rmeta>.rust-js-sites`, as the qualified fact beside a qualified crate's
is. A consumer's lowering reads them (`recognition::registry_site`) and
gives them, `$jsonIndexMut(cell, key, raisedAt, boundAt)`; the runtime
holds no table. A file is named under `/cargo/registry/src/`, whatever the
machine's `$CARGO_HOME` (ADR 0262's exception, the user's decision).
`serde/build.sh` builds the tests' crates through rust-js's Cargo route
(`serde/cargo.ts`), the one way a host builds, so its facts are written the
same way. A crate whose build noted none, or whose source doesn't place a
site, is refused where its panic would be raised, saying why.
indexmap's `get_disjoint_mut` tracks its caller, so it names the caller's
line, as native Rust does.
