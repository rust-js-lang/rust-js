# 0077. `#[derive(Serialize)]` is a function that writes serde_json's text

Status: Accepted. Extends [0052](0052-std-trait-impls.md) and [0060](0060-debug.md).
Amended below (#450): the attributes are found by each node's `DefId`.

Case: C, B, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A full-stack Rust app shares its types between the server and the client:

```
 shared.rs  ── #[derive(Serialize, Deserialize)] struct Order { .. }
   │                                         │
   ▼ rustc                                   ▼ rust-js
 server (axum)  ◄──── JSON over HTTP ────►  client (JS)
   serde_json                                 ???
```

The server writes JSON with serde_json. For the two ends to agree, the
client has to write the very same text: the same names after
`rename_all`, the same enum tags, the same number layout. And it has to be
written from the same source, `#[serde(..)]` attributes and all, or the
two will drift.

serde's derive makes a generic `serialize` that drives a `Serializer`
through a visitor. That code is traits, associated types and generics all
the way down: none of it is JS a person would write, and most of it is
what rust-js rejects.

## Decision

**The crate uses real serde.** `serde/build.sh` builds serde, serde_derive
and serde_json with rust-js's pinned toolchain, and prints the `--extern`
flags that find them. rustc checks the shared crate just as the server's
build does, so a mistake in an attribute is rustc's error.

**What serde's derive generates is skipped; what it means is read from the
attributes.** Every item the derive expands to is left out, and its
`Serialize` impl's `serialize` becomes a function that writes the value
with `$jsonWriter`:

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub order_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
serde_json::to_string(&order)
```

```js
function orderSerialize_serialize(order, json) {
  json.beginObject();
  json.key("orderId");
  json.int(order.order_id);
  if (order.note != null) {
    json.key("note");
    json.string(order.note);
  }
  json.endObject();
}
$toJson(order, orderSerialize_serialize, false);
```

- **It's named and pruned as a derived `Debug` is** (ADR 0060): one
  function per type, emitted only when something serializes it.
- **`serde_json::to_string` and `to_string_pretty`** are `$toJson`. It
  returns a `Result`, whose error is a `serde_json::Error`:
  `{ message, line, column }`, shown as serde_json shows it.
- **The attributes are read before rustc drops them.** HIR keeps no trace
  of `#[serde(..)]`, which only serde_derive reads, so rust-js reads them
  from the expanded AST, by each item's, variant's and field's `DefId`
  (amended below, #450).
- **The attributes supported** are `rename`, `rename_all`,
  `rename_all_fields`, `tag`, `content`, `untagged`, `transparent`,
  `skip`, `skip_serializing` and `skip_serializing_if`. `rename_all`'s
  rules are serde_derive's own, ported as they are. Those that only change
  deserializing (`default`, `alias`, `deny_unknown_fields`, ..) are
  accepted. Any other (`flatten`, `with`, ..) is an error, as is a generic
  type deriving `Serialize`.
- Field skips apply to tuple structs and tuple variants as well as named
  fields. Custom skip predicates use rustc's resolved function from the
  derive, preserving module scope and import aliases.
  Each is called as serde_derive calls it (amended, #1543): a struct, a
  tuple struct or a variant of them counts what it writes first,
  `serialize_struct(name, len)`, calling each field's predicate in turn
  before any field is written, then again as each is; once only where a
  field is flattened, which serde writes as a map of no length. A crate's
  own predicate may answer the second call differently, so the JSON
  depends on it (ADR 0262: results change, so Rust's behavior is kept);
  std's (`Option::is_none`, `is_empty`) only ask what they're given, so
  their count is nothing and `Order` is written as before.
- A variant's `untagged` overrides its container's tagging. Named structs
  include their own `tag` and serialized type name. Internally tagged
  newtype variants unwrap transparent/newtype payloads before writing fields.
  Only where serde's derive writes the payload's fields (amended, #1548):
  a payload whose `Serialize` is written by hand, or goes through
  `#[serde(into)]`, is given serde's tagged serializer, which isn't made,
  so it's refused (#1540); its fields were written instead, `source`
  where its own impl writes `wire`. Results change, so the program is
  refused (ADR 0262).
  **Amended again (#1540): written as serde's `TaggedSerializer` writes
  it.** The payload is written by its own writer, given the adapter a
  flattened field's is (`$jsonFlat`, one adapter, told the variant it's
  `within`): `json.flat(value, writer, "E::A")`. A struct's, a map's
  (`HashMap`, `BTreeMap`, `serde_json::Map`, a `Value`'s object), a
  `Box`'s or a newtype's of one, its entries after the tag; `()` and a unit
  struct, the tag alone; a variant, an entry of its name; anything else,
  serde's own error, verbatim, `cannot serialize tagged newtype variant
  E::A containing an integer`, by the enum's and variant's Rust names. An
  `Option`, which no writer tells from `()`, is that error from its type,
  through a `Box`, an `Rc`, a newtype or a `transparent` struct, returned
  as native returns it. A map was walked as std's struct, its private
  `base`, and a `Box`'s or a `Vec`'s panicked the compiler. Results change,
  so Rust's behavior is kept (ADR 0262). Refused, naming #1568: a type
  parameter's payload and one serde's derive doesn't write, whose `Option`
  its type can't tell, which #1568's markers (B) are for.
  The wire example's `Event::Placed(Order)` is one call of `Order`'s
  writer where it wrote `Order`'s fields again, 22 lines to 1; its renames
  are as many, so the readability gate's `renamed x$1` share moves from
  485.6 to 485.7 in 10,000 lines by the lines gone alone, blessed for it.

**Each type is written as serde writes it:**

| Rust | JSON |
|---|---|
| `bool`, integers | `true`, `7` |
| `f64` | serde_json's layout: `1.0`, `1e-7`, `1e16`; `NaN` is `null` |
| `String`, `&str`, `char` | a JSON string |
| `Option<T>` | `null`, or the `T` |
| `Vec`, slices, arrays, sets, tuples | an array |
| `BTreeMap`, `HashMap` | an object; a number or `bool` key is quoted |
| unit struct, `()` | `null` |
| newtype, `transparent` | what it holds |
| tuple struct | an array |
| enum | externally tagged, or as `tag`, `tag` + `content`, `untagged` say |

A float's text is the one thing JS and serde_json write differently:
`String(1)` is `"1"`, where serde_json's is `1.0`, and they switch to an
exponent at different sizes. `$jsonNumber` takes JS's shortest digits,
which are the same digits, and lays them out as serde_json does.

## Why

- **It's serde_json's answer.** The example serializes every tagging,
  renames, skips, maps, escapes and floats (`NaN`, `-0.0`, `1e16`), and
  its compact and pretty text match native serde_json byte for byte.
- **One source.** The attributes the server's derive reads are the ones
  the client's JS is written from.
- **It's what a person writes** to put a type into JSON by hand, one key at
  a time, with nothing in between.

## Alternatives

- **Compiling serde's generated code.** Faithful by construction, but it's
  a trait-driven visitor: pages of JS for each type, and generics rust-js
  doesn't support.
- **`JSON.stringify` of the JS value.** Short, but the JS value isn't the
  JSON: an enum is `{ TAG, _0 }`, an `Option` is `undefined`, a map is a
  `Map`, and the names are Rust's.
- **A schema shared by both ends** (OpenAPI, protobuf): a second source of
  truth for types Rust already has.

## Consequences

- The playground can't load proc macros, so a crate that derives
  `Serialize` compiles only with the native compiler.
- A `HashMap` is written in JS's insertion order, not Rust's hash order.
  Both are valid JSON objects, but not the same text; use a `BTreeMap`
  when the text must match.
- Reading JSON back (`serde_json::from_str`) is [ADR 0078](0078-serde-json-reading.md).

`test/serde.test.ts` compiles the same source natively and through rust-js,
then compares compact and pretty JSON. It covers these attribute combinations
and predicate resolution independently of generated-code snapshots.

## Amendment: the attributes are found by `DefId`, under `-C incremental` too (2026-11-01, #450)

`rust-js build` compiles as Cargo does, with `-C incremental`, under
which rustc keeps each HIR span relative to its item. The attributes were
kept by the span of the name in the expanded AST and found by
`def_ident_span`, which then never matched: every `#[serde(..)]` was lost,
`{"order_id":1,"note":null}` where serde_json writes `{"orderId":1}`, and
`{"Waiting":{..}}` for `{"kind":"waiting",..}`. A wrong answer, of a
build's default flags (P0).

**Each item's, variant's and field's `#[serde(..)]` is kept by its
`DefId`**, the resolver's for its node (`PerOwnerResolverData`'s
`node_id_to_def_id`, its item's own `def_id`), and found by the `DefId`
rustc gives the type, its variant or its field. No span is matched, so
nothing depends on how rustc keeps one. Case C (ADR 0262): the JSON is
serde_json's.

Compiling the corpus with and without `-C incremental` (1,350 cases)
gives the same JS for each, so no other analysis matched a span across
that boundary.

Held by `test/serde.test.ts`'s "#[serde(..)] holds under -C incremental,
its cache cold and warm", against native serde_json, the one test that
compiles with `-C incremental` other than Cargo's builds, and the
mutations `serde-item-def-id-unfound` and `serde-field-def-id-unfound`
(`lower/serde.ts`).

## Amendment: a read of what runs code when it's dropped is refused (2026-11-12, #1552)

A read that fails part-way, a field's text the wrong type, trailing
characters, a duplicate field, drops what it had read in Rust, as `?`
returns through each of serde_json's frames: `drop 1` before the `Err` of
`from_str::<S>(r#"{"a":1,"b":"x"}"#)`. The runtime reader's frames hold
what's read, a struct's values, a `Vec`'s items, and drop nothing as its
`$JsonError` is thrown, so rust-js printed the `Err` alone. #1532's plan
moves the reader to serde's own expansion, where Rust's frames are
rust-js's lowered ones; until then a read of a type whose drop can be seen
is refused, its preferred S1 answer: ADR 0262's last case, as the runtime
frames can't drop what they hold. One question, `refuse_dropping_read`
(`serde/de.rs`), asked where a read begins: `from_str`, `from_value`, a
hand-written impl's `T::deserialize(d)`, and a visitor's `next_element`,
`next_key`, `next_value` and `next_entry`. A drop is seen (`drop_seen`)
where the type runs code as it's dropped (`drops`, the drop owner's
answer) by a `Drop` of the crate's whose call can be seen, not one that
does nothing (`quiet_fns`), a library's, or one of what only a caller
knows; std's own containers drop only what they hold. #1552 stays open,
P2, for the reader that drops.

Held by test/diagnostics.test.ts (a struct of such fields from text, a
`Vec` of them from a `Value`, a visitor's element), test/serde.test.ts's
generic codec of a value whose `Drop` does nothing, read as before, and
mutations `dropping-read-unrefused`, `quiet-drop-taken-as-seen` and
`std-container-drop-taken-as-seen`.

## Amendment: refused, not a panic: a serde_json method and `remote` (2026-11-13, #1541)

serde_json's free functions are recognized by their names, `from_str`,
`to_string` and `to_string_pretty`; a method of the same name,
`Deserializer::from_str`, was taken for `serde_json::from_str`, and the
compiler panicked on its arguments. They're its free functions alone now,
and the method is refused as any call rust-js has no model of is.
`#[serde(remote = ..)]`, whose derive writes inherent functions of a
remote type's stand-in that no codec of rust-js's reads, panicked where a
field's `with` named one; each is refused before anything is lowered
(`serde::refuse_unread`, #1615). `test/diagnostics.test.ts` holds both.
Mutations `serde-json-method-taken-for-its-function` and
`serde-remote-unrefused`, caught.

