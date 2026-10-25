# 0078. `serde_json::from_str` is serde_json's reader, ported; `#[derive(Deserialize)]` is a table it reads by

Status: Accepted. Extends [0077](0077-serde-json.md).

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

ADR 0077 made the client write what the server reads. The other way, the
client has to read what the server writes, and do it as serde_json does:
take what serde_json takes, turn down what it turns down, and say why in
its words, at its line and column. A client that shows "missing field
`orderId` at line 1 column 19" should show what the server's own tests
would.

`JSON.parse` doesn't: it takes a duplicate key, turns a `u64` into a
rounded float, and its errors are the engine's. And serde's derive writes a
visitor full of traits and generics that rust-js doesn't compile.

## Decision

**`$JsonReader` is serde_json's `Deserializer`, ported step for step,** with
its methods' names (`parseInteger`, `parseDecimal`, `endSeq`, `ignoreValue`
..). It reads the text's UTF-8 bytes, as serde_json does, so a mistake is
found at the same byte, and a column counts bytes, as serde_json's does:

- **Its messages are serde_json's**, and serde's: `invalid type: string
  "7", expected u32`, `unknown field`, `missing field`, `duplicate field`,
  `invalid length`, `trailing comma`, `recursion limit exceeded`, with
  serde_json's own words where it has them (`null`, not `unit value`; a
  float as it writes one).
- **Its places are serde_json's.** Where serde_json puts an error from a
  visitor where the reader is (`fix_position`), so does it, and it looks
  for a closing bracket after a failure, as serde_json does, since that
  moves where the reader is.
- **Its numbers are serde_json's:** an integer is exact to 64 bits (a
  BigInt past 2^53) for its messages, and a float is the significand times
  or over a power of ten, the way serde_json computes one without its
  `float_roundtrip` feature, which isn't always the nearest `f64`.

**A derived `deserialize` is a table of what serde's visitor knows**: each
field's name, how to read it, and what it is when it's missing. The reader
does the rest, as the visitor would:

```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub order_id: u32,
    #[serde(default)]
    pub qty: u32,
    pub note: Option<String>,
    #[serde(skip)]
    pub cache: u32,
}
```

```js
function orderDeserialize_deserialize(json) {
  return json.struct(
    "struct Order",
    [
      ["orderId", $json.u32],
      ["qty", $json.u32, () => 0],
      ["note", $json.option($json.string)],
    ],
    ([order_id, qty, note]) => ({ order_id, qty, note, cache: 0 }),
  );
}
```

- **What reads a type** is `$json.u32`, `$json.vec($json.string)`,
  `$json.map($json.key.string, $json.f64)`, or the crate's own type's
  `deserialize`: what serde's impl for it reads, with its name for the type
  (`u32`, `a sequence`, `a tuple of size 2`).
- **A missing `Option` is `None`,** as serde's `missing_field` makes it: the
  field is read from a reader that has nothing but `null`.
- **The attributes read** are `rename` and `rename_all` (each side of
  `serialize = .., deserialize = ..`), `rename_all_fields`, `alias`,
  `default` and `default = "path"` on a field or the container, `skip` and
  `skip_deserializing`, `deny_unknown_fields`, `transparent`, `other` and
  `expecting`.
- **An enum is read externally tagged:** `"Dot"`, or `{"Circle": 1.5}`, by
  its variants' names, then as its variant holds: nothing, a value, a
  tuple or a struct. Its other representations are [ADR
  0079](0079-serde-tagged-enums.md).
- `serde_json::from_str` is `$fromJson(text, read)`: a `Result` of what
  `read` reads, if it's all the text holds.

**A derived codec is lowered only when it's used.** A shared crate derives
`Serialize` and `Deserialize` for its types; a client that never reads an
untagged enum shouldn't fail on its `deserialize`. So a derived
`serialize` or `deserialize` is lowered once something that's lowered calls
it, a round at a time, in the order they're declared.

## Why

- **It's serde_json's answer.** The `inbox` example reads structs, enums,
  maps, sets, tuples and arrays, with every attribute above, from right and
  wrong JSON (bad escapes, numbers, commas, keys, lengths, depths), and
  every value and error matches native Rust to the column. So do 2,000
  number texts read as an `f64` and an `i32`.
- **Ported, not rewritten:** each step can be checked against serde_json's
  own, by its name.
- **The table is short, and it's all the type's own:** the protocol, the
  same for every type, is written once.

## Alternatives

- **`JSON.parse`, then checking the value.** Short and fast, but it takes
  what serde_json doesn't, loses a big integer's digits, and can't say
  where a mistake is.
- **A visitor per type,** as serde's derive writes: the most literal, but
  each struct would be dozens of lines of the same loop.
- **Correctly rounded floats** (`Number(text)`): nearer, but not what the
  server reads from the same text.

## Consequences

- A `&str` is read as serde_json reads one, borrowed from the text: a
  string with an escape in it is `invalid type: .., expected a borrowed
  string`. A value kept for a tagged enum (ADR 0079) keeps whether it
  could be borrowed, as serde's `Content::Str` does.
- A `BinaryHeap`, `u64`, `i64` and `f32` can't be read yet; each is an
  error.
- `usize` is 32 bits, as on wasm32 (ADR 0025): `5000000000` is too big
  for it, as it would be there.

## Amendment: what's malformed is held against serde_json

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a text's value, or
its error with its words, line and column, is serde_json's.

The reader is serde_json's `Deserializer`, ported step for step, 722
lines, and one mutation held it: a float's rounding. No test named any of
its errors (an architecture review, 2026-10-25), so a port of an escape,
a surrogate, a bound or a position could go wrong unseen.

- **`test/serde.test.ts`'s "text, well formed or not, is read and refused
  as serde_json reads it"** reads 148 texts, and brackets 129 deep and
  127, each as 26 types: `Value`, `String`, `Vec<u32>`, `i64`, `u64`,
  `f64`, a struct with an optional field and one that denies unknown
  fields, a tuple, an `Option<u32>`, an externally, an internally and an
  adjacently tagged enum, an untagged one, structs with a flattened map,
  struct, `Option`, number, internally tagged and untagged enum, maps keyed
  by numbers and booleans in quotes, enums read from what an internally
  tagged variant buffered and one flattened, and a visitor of a map's first
  entry alone, which a `Deserialize` written by hand may be. Each text that
  is a `Value` is read again from it, `from_value`, as 24 of them. Each
  gives what serde_json gives natively, value or error.
- **Its mutations, 21 of the reader's, 16 of what reads a sequence, a
  map, a key, a variant and buffered content (`json_content.js`), 11 of a
  `Value` read as a deserializer (`json_value_reader.js`) and 7 of what's
  flattened (`json_flat.js`)**, are each a rule, all caught: trailing text, the depth of 128, an error's
  line and column and the byte a `peek_error` is at, a leading zero, `-0`
  a float and `i64::MIN` an integer, `1.` at the end, a float past
  `f64::MAX` by its digits, control characters, `\q` and `\uZZZZ`, lone
  and unpaired surrogates, a trailing comma in a list and in a map a
  visitor left early, the colon, tab and carriage return as whitespace, an
  ident cut short; a list's and a map's commas and keys, a tuple's length,
  a buffered list with items over, number and boolean keys in quotes, a
  buffered enum's single key and a variant by its index, a unit variant
  given for one that holds something, and a buffered variant of the wrong
  shape; a `Value`'s array with items over, its object read by key, as
  serde_json's `BTreeMap` is, its keys, variants and `null`s; and a
  flattened struct's fields taken, an `Option` of none, an enum's variant
  taken and found, and what's left for an untagged enum.
- **What it found**: a `Deserialize` written by hand, given buffered
  content, an untagged variant's or an adjacently tagged one's whose
  content comes first, or a `Value` by `from_value`, throws a `TypeError`
  (#236). The texts that reach it, and the mutations of a buffered map's
  and a `Value`'s object's entries left over, which only such a visitor
  reaches, wait for its fix.
