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
   and `end()` its bracket. Each gives `Ok` where the impl gives it on by
   `?` or returns it, else its `Result` (amended below, #1545); a
   compound's `&mut self` is a box in generic code (ADR 0454), the writer
   in it.
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

- A visitor's `visit_enum`, and `deserialize_enum`, are refused yet.

## Amendment: a hand-written `Visitor`

A `Deserialize` that reads by its own `Visitor`, `d.deserialize_any(LevelVisitor)`,
was refused. Its visitor is given what's there, as serde_json gives it:

```js
function levelDeserialize_deserialize(d) {
  return $jsonVisit(d, () => levelVisitorVisitor_expecting(undefined), {
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

**Amended (#1536): at each position, used or refused, never ignored.**
The places that wrote or read a field on its own never asked
`field_codec`, so serde_json wrote and read another JSON with no error: a
newtype struct's field and a `transparent` one's, a newtype variant's in
each tagging, a flattened field's, which is given the entries (`flat`'s
writer, the flattened reader), and the attribute on a variant itself,
which serde gives a newtype variant's one field (`newtype_variant_codec`).
On a variant of no field or of more, serde gives the function all of
them, which isn't made, so it's refused; so is a writer of an internally
tagged newtype variant's field, or of the newtype struct it holds, which
serde gives its tagged serializer. Results change, so Rust's behavior is
kept or the program refused (ADR 0262). `test/serde.test.ts` writes and
reads each position against native serde_json; `test/diagnostics.test.ts`
holds the refusals; a mutation drops each read.

## Amendment: collections, variants and an option

- **`collect_seq(items)` and `collect_map(entries)`** are
  `$jsonCollectSeq` and `$jsonCollectMap`: each item, or each entry's key
  text then its value, written in turn by its writer, of what's given as
  a JS iterator, as `collect` would step it.
- **`serialize_tuple_variant` and `serialize_struct_variant`** begin an
  object of the variant's name around their array or object, and their
  `end()` closes both. A tuple struct's or a tuple variant's
  `serialize_field(&v)` is an element, a struct's `serialize_field(key,
  &v)` a key and a value: recognition tells them by their trait.
- **`deserialize_option`** is `$jsonVisitOption`: `null` to the visitor's
  `visit_none`, anything else to its `visit_some`, given the reader; of
  one it hasn't, serde's "invalid type: Option value", with no place, as
  serde_json gives it, but where a struct reading it places it. A method
  that takes nothing, `visit_none` and `visit_unit`, is given nothing.

The serde tests of a hand-written `Serialize`'s collections and variants,
and of a visitor's option, against native serde_json.

## Amendment: a hint given a value already read (#236)

A hint's `deserialize_map(visitor)` of a hand-written visitor peeked the
JSON text, `reader.parseWhitespace()`, to refuse a kind the visitor has no
method for, as serde_json's `peek_invalid_type` does. Given serde's
buffered content, an untagged variant's or an adjacently tagged one's
before its tag, or a `serde_json::Value` through `from_value`, there's no
text. It threw `TypeError: reader.parseWhitespace is not a function`,
where serde_json reads it. A wrong answer, so ADR 0262's "results change"
case: Rust's behavior is kept.

Each decoder now answers it, `deserializeHinted(expected, visitor)`, given
the methods that hint takes. The text's peeks first, `$JsonReader`'s. A
value already read, `$JsonDecoder`'s, is visited by its kind's method, or
refused as the type it is, as serde's `ContentDeserializer` refuses it.

- The malformed-text table in `test/serde.test.ts` has back the rows it
  left out, `{"c":{"a":1,"b":2},"t":"F"}` among them. It has an untagged
  `EitherFirst`, and `First` read from a `Value`, all against native Rust.
- The mutations are `hinted-content-peeked`, `hinted-text-unpeeked` and
  `json-content-map-left-over-kept`, put back.

## Amendment: a protocol call's `Result` is what the impl does with it (2026-11-12, #1545)

Each protocol call gave `Ok` and threw its error (decision 2, "Each gives
`Ok`"): exact where the impl only gives the `Result` on by `?`, or returns
it, and wrong where it looks at it. `match seq.serialize_element(&Bad) {
Ok(()) => seq.end(), Err(_) => Err(custom("caught")) }` gave serde_json's
`inner` where native Rust gives `caught`: the `Err` arm was never reached.
Agent-fable's first rule for the protocol layer (#1532): a protocol call's
`Result` is a control-flow fact, decided at its use site.

- **One owner, `mir::result_use`** (`src/lower/mir/result_use.rs`), moved
  from the writer's `given_on` (#1524), which a `write!` into a
  `Formatter` asks too: a call's `Result` is `Propagated` where it's the
  body's own result, or a local read once, by `?`'s `Try::branch`, in a
  body whose own error type is the call's, so `?`'s `From` changes
  nothing; else `Inspected`. A drop of it a cleanup makes isn't a look at
  it. test/architecture-mir.test.ts holds the readers of `Try::branch` to
  the files that do now, and that both ask `result_use`.
- **`Propagated`, as before**: the steps, `Ok`, an error thrown. No `?`
  code changes, so derived codecs and the corpus are byte for byte as
  they were, the guide's `Order` too: #1532's cost gates.
- **`Inspected`: `$jsonTry(() => { ..steps..; return v; }, writer)`**,
  which gives `Ok(v)`, or `Err(e)` of the backend's `$JsonError` alone;
  any other throw, a panic, goes on. And it takes the writer back to
  where the call began (`mark`, `rewind`): the text's writer keeps what it
  wrote, `[1,` of a failed element, as serde_json's keeps its bytes; a
  `Value`'s drops what it opened of the value it failed in, as serde_json
  builds each value apart and adds it once it's made. `$jsonRead`, of a
  `T::deserialize(d)`, is `$jsonTry` of the read.
- An error given on by `?` into another error type, `At<S::Error>` by its
  `From`, is `Inspected`: the call's `Result`, then `?`'s conversion as
  Rust writes it.

Proof: test/serde-protocol.test.ts, against native serde_json through
`to_string` and `to_value`: a `match` on `serialize_element`'s `Err`, an
`is_err()` per element written on after, a nested impl's partial text
kept, `map_err` of a struct's field, `let _ =` of a map's entry, a nested
`serialize`'s `Result` matched, `?` through `From` into another error, and
a panic under an inspected call, still a panic. Mutations
`inspected-protocol-result-thrown`, `inspected-protocol-writer-unrewound`,
`json-try-catches-panics`, `value-writer-unrewound`,
`result-use-always-propagated`, `result-use-conversion-unasked`,
`result-use-drop-taken-for-a-use`; `tried-write-result-held` and
`last-write-result-held` moved to `result_use.rs`.

## Amendment: a visitor's `expecting` is asked where an error says it (2026-11-13, #1544)

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change, so
Rust's behavior is kept.

`$jsonVisit`, `$jsonVisitHint` and `$jsonVisitOption` were given the
visitor's `expecting` called as the read started,
`levelVisitorVisitor_expecting(undefined)`, its text. serde never asks it
of a read that succeeds: it's a `&dyn Expected`, formatted only where an
error's message is made, each time one is. So a successful read ran
`expecting`'s side effects, a `println!`, a count, and its panic, where
native ran none (P0, both builds, both engines).

Now the visitor's `expecting` is given as a function, `() =>
levelVisitorVisitor_expecting(undefined)`, and what's expected is
`$jsonExpected(expecting)`, `{ toString: expecting }`: text made of it each
time it's written into a message, the readers' `${expected}`, as serde
formats a `&dyn Expected`. A `&dyn Expected` the impl makes itself, its
`invalid_type(.., &self)`'s, is still its text, made as the impl's error
is, once, as serde's `Error::invalid_type` formats it.

The serde-protocol test `a visitor's expecting is asked only where an
error says it` counts each call of `expecting` of `deserialize_bool`,
`_any` and `_option`, from text and from a `Value`, a field's too, and
reads a visitor whose `expecting` panics, against native serde_json; a
`Value`'s `deserialize_option` of a hand-written visitor throws, #1597,
so that one reads text alone. Mutations `serde-expecting-asked-eagerly`
(`lower/serde/protocol.ts`) and `json-expected-made-once-up-front`
(`runtime/json_visit.ts`), caught.

## Amendment: a string is given by the kind serde_json gives it (2026-11-13, #1546)

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change, so
Rust's behavior is kept.

A visitor's `visit_string` wasn't one of the methods `visitor_methods`
read, and the readers said of a string only whether it borrowed from the
text. So `from_value` of a visitor that writes `visit_str` and
`visit_string` called `visit_str`, where serde_json gives a `Value`'s
string whole, `visit_string`; an internally tagged enum's content, which
serde gives whole too, likewise; and `deserialize_any` of an untagged
enum's content gave no string borrowed, `visit_str` for one serde gives
`visit_borrowed_str` (P0, both builds).

Now a reader gives a string with one kind, never a bool:

| Read from | Kind | The visitor's method |
|---|---|---|
| the text, no escape | `borrowed` | `visit_borrowed_str`, else `visit_str` |
| the text, an escape | `transient` | `visit_str` |
| a `Value` | `owned` | `visit_string`, else `visit_str` |
| content by reference, an untagged enum's | `borrowed` where it borrowed the text, else `transient` | as above |
| content whole, an internally tagged enum's | `borrowed` where it borrowed the text, else `owned` | as above |

The fallbacks are serde's defaults, `visit_borrowed_str` and
`visit_string` forwarding to `visit_str`, whose default is "invalid type".
`visitor_methods` reads `visit_string` (`VisitorMethod::String`); a
derived impl's output, which takes no kind, is as it was, and so is the
std `&str`'s borrowed check (`borrowedStr`, `kind === "borrowed"`).

The serde-protocol test `a string is given to a visitor as serde_json
gives it` holds the native table of `deserialize_string`, `_any` and
`_str`, a visitor of `visit_string` alone, fields, an untagged and an
internally tagged enum, each from text and from a `Value`, against native
serde_json. (serde cases live in the serde tests, which build serde and
serde_json natively, not in the corpus.) Mutations, one of each kind's
mapping: `json-visit-owned-as-str`, `json-visit-borrowed-as-str`,
`json-visit-transient-as-owned` (`json_visit.ts`),
`json-reader-escaped-borrowed`, `json-value-string-by-reference`,
`json-content-owned-by-reference`, `json-content-reference-owned`,
`json-content-any-kind-lost` and `serde-visit-string-unwritten`, caught;
`json-borrowed-str-of-escape` and `json-visit-copied-as-borrowed`
refitted.

## Amendment: a 128-bit integer is written by each serializer's own rule (2026-11-13, #1547)

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change, so
Rust's behavior is kept.

Recognition took `serialize_i128` and `serialize_u128` as any other
integer's, `SerdeScalar::Int`, the writer's `int(n)`, and a `Value`'s
writer holds any BigInt. So `serde_json::to_value` of an impl that writes
`u128::MAX` was `Ok(Number(340282366920938463463374607431768211455))`, a
`Number` serde_json can't make, where native gives `Err(Error("number out
of range", line: 0, column: 0))` (P0, both builds). The text and `Value`
serializers aren't one: each has its own rule for a width.

Now the scalar keeps its width, `SerdeScalar::Int { wide }`, `Some("i128")`
or `Some("u128")` of a 128-bit one, and its writer is told it,
`s.int(n, "u128")`, as an `f32` is told `single`. Each writer is serde's:

| Writer | A 128-bit integer |
|---|---|
| text, `$jsonWriter` | its digits, as before |
| a `Value`, `$toJsonValue` | `$jsonWideInt`: a `Number` where it fits in an `i64` or a `u64`, else `number out of range` |
| flattened or internally tagged, `$jsonFlat`, at its top | serde's default, which these don't override: `u128 is not supported`, `i128 is not supported` |
| within a flattened map | the object's own writer's |

A derived field of 128 bits stays refused, read and written ([ADR
0171](0171-128-bit-integers.md)). The serde-protocol test `a 128-bit
integer is written by each serializer's own rule` holds both ends of
each width, `u64::MAX` and one past, `i64::MIN` and one past, a failed
element asked and written on after, a sequence, an untagged enum and the
flattened cases, to text and to a `Value`, against native serde_json.
Mutations `value-wide-int-unchecked`, `value-wide-int-signed-range`
(`json_value.ts`), `flat-wide-int-said-as-integer` (`to_json.ts`),
`serde-u128-width-lost` (recognition) and `serde-wide-int-ungiven`
(protocol), caught.

## Amendment: the readers are one interface, and `deserialize_option` is each one's (2026-11-13, #1549, #1597)

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change, so
Rust's behavior is kept.

`$jsonVisitOption`, a hand-written visitor's `deserialize_option`, peeked
for `null` with the text reader's own `parseWhitespace()`. A `Value`'s
reader has none, so `from_value` of such a visitor threw `TypeError:
reader.parseWhitespace is not a function` (P1, both builds); serde's
buffered content, an untagged or internally tagged enum's, likewise. The
readers had no stated interface, so nothing caught a helper calling one
reader's own method of any reader.

Each reader now writes `deserializeOption(expected, visitor)` by serde's
own rule for it:

| Reader | `null` | anything else |
|---|---|---|
| the text, `$JsonReader` | `visit_none` | `visit_some`, given the reader |
| a `Value`, `$JsonValueReader` | `visit_none` | `visit_some` |
| serde's content, `$JsonContent` | `visit_unit`, a unit, else `invalid type: null` | `visit_some` |
| a missing field, `$JsonMissing` | `visit_none`, serde's `MissingFieldDeserializer` | |
| a flattened field, `$JsonFlat` | `can only flatten structs and maps` | the same |

`$jsonVisitOption` asks it, and an option hint's visitor gives its
`visit_unit` too. A method the visitor lacks is serde's `invalid type:
Option value`. The interface is held by `test/architecture-json.test.ts`:
each method the code over a reader calls of it, `$JsonDecoder`'s of
`this`, `json_visit.js`'s of `reader` and the `$json` codecs' of `json`,
is written by `$JsonReader`, `$JsonValueReader` and `$JsonContent` or
inherited from `$JsonDecoder`; a planted `reader.parseWhitespace()` is
caught. Every program that reads JSON carries the readers' classes, which
grow by these short methods; codegen and the guide's Order output don't
change.

The serde-protocol test `a visitor's deserialize_option is each reader's
own` holds a visitor of `visit_none` and `visit_some`, one with
`visit_unit` too and one of `visit_some` alone, each from text and from a
`Value`, in an untagged and an internally tagged enum, as struct fields,
missing and flattened, and in a `Vec`, against native serde_json.
Mutations, one of each reader's rule: `text-option-null-as-unit`,
`value-option-null-to-some`, `missing-option-field-refused`,
`content-option-unit-as-none`, `flat-option-read`,
`visit-option-none-unchecked` and `serde-option-unit-unwritten`, caught.
`T::deserialize(&value)` of a `&Value` is #1539's, apart.

## Amendment: `T::deserialize` of a `Value` or a `&Value` (2026-11-13, #1539)

Case ([0262](0262-when-rust-and-js-disagree.md)): R, the JS threw where
Rust runs.

A `serde_json::Value` is a `Deserializer`, and so is a `&Value`:
`T::deserialize(value)` reads a `T` from it, as `from_value` does. The
call gave `T`'s reader the `Value` itself, `json.struct is not a function`
(P1). Its deserializer is now read by its type (`protocol::deserializer`).
A type parameter's is a reader already. A `Value`'s is `new
$JsonValueReader(value)`, whose strings are given whole, `visit_string`. A
`&Value`'s is `new $JsonValueReader(value, "borrowed")`, which lends them,
`visit_borrowed_str`, as each of what it holds does, an array's items, an
object's values and a variant's payload (the reader's `kind`, #1546's
table's row of a `Value` now of a `&Value` too). Generic code's is its
caller's reader already.

The serde-protocol test `T::deserialize of a Value or a &Value reads T
from it` holds a derived struct, a number, a `Vec`, an error, a hand-written
visitor's strings of each, an `Option`, a hand-written impl reading a
`&Value` it indexes, and an enum's payloads, against native serde_json.
Mutations `value-deserializer-unread`, `value-reference-deserializer-owned`
and `value-seq-kind-unkept`, caught.


## Amendment: a library's hand-written impl, compiled by its consumer (2026-11-13, #1560)

A hand-written `Serialize` or `Deserialize` is generic over serde's
`Serializer` or `Deserializer`, so its library's JS doesn't export it
(ADR 0914): a consumer that serializes the library's type compiles its own
copy, from the MIR the library's metadata keeps, as it does any of a
library's generic functions it calls. Two places asked only whether the
library exported it, and refused: a type's codec (`codec_fn`) and a
hand-written `Deserialize`'s `Visitor` (`serde_visitor_impl`). Each now
asks `Foreign::compiled`, the one answer to whether a library's item has
JS the crate calls, exported or compiled here. No contract field: the
consumer reads the impl's body from rustc's metadata, as it does every
other generic function's. Case ([0262](0262-when-rust-and-js-disagree.md)):
no observable difference, the same codec run as one crate's.

Proof: the crate graph's `models` gains `Celsius`, a hand-written
`Serialize` and a `Deserialize` with its own `Visitor`, used by
`frontend` against native (`test/crates.test.ts`, "an app using two
crates"); the serde probes' split mode, 19 refused before now native's
answer, `manual_*`, `issue_1545`, `issue_1546` and `drop_01` among them,
and all three modes now the same, 277 native's, 89 refused, none
different; two mutations.
