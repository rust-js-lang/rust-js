# 0459. A hand-written `Serialize` or `Deserialize` is its body, over the JSON codec

Status: Accepted. Extends [0077](0077-serde-json.md),
[0078](0078-serde-json-reading.md) and [0081](0081-serde-bounds.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same JSON, and
the same errors at the same places.

## Context

A shared type with a format of its own writes its serde impls by hand:
money as `"12.34"`, a date as text, a struct whose fields depend on its
state. chrono, uuid, rust_decimal and url each do.

rust-js wrote every serde impl as a derived codec, by the type's
`#[serde]` attributes, and lowered a hand-written impl's body too: two
functions of one name, the codec reading a `Cents` as if it derived
`Deserialize`, and the body calling `$json.string.deserialize(d)`, which
doesn't exist. Its serializer's `serialize_str`, and `de::Error::custom`,
were refused.

## Decision

**A derived impl is a codec rust-js writes; a hand-written one is its
body, as Rust wrote it, over the JSON codec: its serializer is the JSON
writer, its deserializer the reader.**

```rust
impl Serialize for Point {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut state = s.serialize_struct("Point", 2)?;
        state.serialize_field("x", &self.x)?;
        if let Some(tag) = &self.tag {
            state.serialize_field("tag", tag)?;
        }
        state.end()
    }
}
```

```js
function pointSerialize_serialize(point, s) {
  s.beginObject();
  const state = s;
  state.key("x");
  state.int(point.x);
  if (point.tag != null) {
    state.key("tag");
    state.string(point.tag);
  }
  state.endObject();
  return { TAG: "Ok" };
}
```

1. **Which is which is `serde_derived`**: serde's derive's, by its
   expansion, a library's by `#[automatically_derived]`.
2. **The serializer's calls are the writer's**, `SerdeOp`, recognition's:
   a scalar's, `s.string(x)`; `serialize_some` and a newtype's, the value's
   own writer; a unit variant its name, a newtype variant `{ variant:
   value }`; `collect_str`, what the value shows. A struct, a sequence, a
   tuple or a map it begins is the writer too, a field its key then its
   value, an element `element()` then it, an entry its key then its value,
   and `end()` its bracket. Each gives `Ok`; a compound's `&mut self` is a
   box in generic code (ADR 0454), the writer in it.
3. **`T::deserialize(d)` is `$jsonRead(d, readT)`**: what the reader reads,
   `Ok`, or what it throws, `Err`, with the place serde_json found.
4. **`S::Error::custom(msg)` and `D::Error::custom(msg)`** are an error of
   what `msg` shows, `$jsonError(..)`, which the reader places where
   serde_json does once it's thrown through it.
5. **Where a codec is wanted, a hand-written impl's function is
   `$jsonCodec(f)`**: its `Ok`'s value, or its `Err` thrown, as the
   codec's own errors are.

### Also here

- **`?` of an `Ok` made where it's tried**, `serialize_field(..)?`, and
  `Ok(3)?` as it's written, has no `Err` to give back: tidy's
  `made_ok_tried` takes the branch it does in its place, and what it held
  is read as it is: `const v = 3`, not a test of an object just made.
  `result::made_ok_payload` answers what an `Ok` made here holds, a
  unit's too.
- **A source map's tokens are written in the order of where they are**:
  formatting may move a node before its neighbour's, and the encoder
  panicked on tokens out of order.

## Why

- **It's the program Rust runs**: the serde tests write a struct, a
  sequence, a map, variants, `collect_str`, an `f32`, a generic impl and a
  custom error by hand, and read through `String::deserialize` with
  custom errors, nested in a derived struct, against native serde_json,
  the same text and the same errors at the same places.
- **It reads as a person writes it**: the writer's steps, in the body's
  order.

## Consequences

- A hand-written `Visitor`, and the `Deserializer` calls that take one,
  `deserialize_any` and the rest, are refused yet: the next step.
- `serialize_tuple_variant` and `serialize_struct_variant` are refused yet.
