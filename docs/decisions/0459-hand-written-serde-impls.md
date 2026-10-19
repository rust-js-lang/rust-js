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

- `visit_some` and `visit_none`, and `deserialize_option`, are refused yet.
- `serialize_tuple_variant` and `serialize_struct_variant` are refused yet.

## Amendment: a hand-written `Visitor`

A `Deserialize` that reads by its own `Visitor`, `d.deserialize_any(LevelVisitor)`,
was refused. Its visitor is given what's there, as serde_json gives it:

```js
function levelDeserialize_deserialize(d) {
  return $jsonVisit(d, levelVisitorVisitor_expecting(undefined), {
    u64: (v) => levelVisitorVisitor_visit_u64(undefined, v),
    str: (v) => levelVisitorVisitor_visit_str(undefined, v),
  });
}
```

- **Its methods are its impl's functions**, each of the visitor:
  `visitor_methods` says which of serde_json's it writes.
- **`$jsonVisit` gives each its kind of value**, the reader's
  `deserializeAny`'s: a number to `visit_u64`, `visit_i64` or `visit_f64`
  by its kind, a `u64` or `i64` as a BigInt; a string to
  `visit_borrowed_str`, or `visit_str` where it was escaped, copied; and of
  a kind it has no method for, serde's "invalid type", expecting what its
  `expecting` says, as serde's default `visit_*` says it.
- **A hint is `deserialize_any` of the methods of its kinds**:
  `deserialize_str` gives only strings, `deserialize_u64` only numbers, as
  serde_json's, whose error for anything else is the same.
- **serde's errors are its messages**: `invalid_type`, `invalid_value`,
  `invalid_length`, `unknown_variant`, `unknown_field`, `missing_field`
  and `duplicate_field`, `$jsonUnexpected` showing an `Unexpected` as serde
  does, a float with its decimal point.
- **A `&dyn Expected` is its text**, as a `&dyn Debug` is (ADR 0060): a
  string's itself, a visitor's what its `expecting` says.

The serde test of a hand-written `Visitor` gives one each kind of value
and makes each error, through `deserialize_any`, `deserialize_str` and
`deserialize_u64`, against native serde_json.

## Amendment: a visitor's sequence and map

A visitor that reads a struct by hand, `visit_seq` and `visit_map`, was
refused, and with it `deserialize_struct`, `deserialize_seq` and
`deserialize_map`.

- **Its access is the reader's**, `$JsonSeq` or `$JsonMap`, given where
  serde_json gives one: `seq.next_element::<T>()` is
  `$jsonNextElement(seq, readT, boxed)`, a `Result` of an `Option`, `None`
  past the last, `Some` boxed where `T` looks like `None` (ADR 0051); a
  map's `next_key`, `next_value` and `next_entry` the same, a key read by
  its key reader. Each takes `&mut self`, a box in generic code (ADR
  0454), the access in it.
- **A hint takes its kinds**: `deserialize_seq` and a tuple's a sequence,
  `deserialize_map` a map, `deserialize_struct` either, as serde_json's
  does. Its visitor is its last argument.
- **A hint but `deserialize_any` errs where it peeks**,
  `$jsonVisitHint`: of a kind it has no method for, serde_json's
  `peek_invalid_type`, before it's read, a sequence's or a map's where it
  starts, column 0 of `[1]`, not 1.

The serde test of a hand-written visitor's sequence and map reads a
struct from either, misses, repeats and mistakes fields, and reads a map
by its entries, against native serde_json.

## Amendment: a field's own writer and reader

`#[serde(with = "module")]`, `serialize_with = "f"` and
`deserialize_with = "f"`, a field's own format, were refused as
attributes rust-js didn't read. Each names a hand-written codec, which
the field's writer or reader now is: the function rustc resolved where
the derive calls it, `$jsonCodec(f)`, asked of `field_codec` by each
place a field is written or read, a struct's, a tuple's, a newtype
variant's. Where the derive calls it with `?`, its desugaring's paths,
`branch` and `from_residual`, are under the attribute's span too, so the
path looked for is the one that isn't a desugaring's. The serde test of
a field's own functions writes and reads them, and their errors, against
native serde_json.
