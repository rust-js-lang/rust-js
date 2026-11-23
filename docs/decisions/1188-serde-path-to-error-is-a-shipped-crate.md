# 1188. serde_path_to_error is a shipped crate: the reader keeps where it is

Status: Accepted (2026-11-23, #2379; the lead, m6-react, approved for Tu
that serde_path_to_error be supported natively, idiomatic Rust in, with no
API of rust-js's own). Extends [0529](0529-a-shipped-memchr-searches-bytes.md)
and [1185](1185-getrandom-is-a-shipped-crate.md)'s catalog, and
[0078](0078-serde-json-reading.md)'s reader; takes `Deserializer::from_str`
out of #1626.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so the
crate's own rules are kept, path for path: native serde_path_to_error
0.1.20 over serde_json 1.0.154 is the oracle.

## Context

serde_json says where in the *text* a read failed, `at line 1 column 103`.
A zod user, or anyone whose JSON is nested, wants where in the *value*:
`items[1].qty`. Rust's answer is serde_path_to_error:

```rust
let reader = &mut serde_json::Deserializer::from_str(text);
let order: Result<Order, _> = serde_path_to_error::deserialize(reader);
```

rust-js refused both calls (#1626, and the crate wasn't known). The crate
works by wrapping serde's generic `Deserializer`: every `deserialize_*`,
`SeqAccess`, `MapAccess` and `EnumAccess` call goes through its wrapper,
which keeps a chain of where it is, and the first error to pass a wrapper
records that chain. rust-js doesn't run serde's generic machinery: a
derive is a table its runtime reader reads by (ADR 0078). So compiling the
crate would never see a call to wrap.

## Decision

**serde_path_to_error is a shipped crate, built by rustc and qualified at
0.1.20. rust-js writes its read as the runtime's reader keeping where it
is in the value, and its `Error` as `{ path, original }`, shown as the
crate shows it.**

| Rust | JS |
|---|---|
| `serde_json::Deserializer::from_str(text)` | `new $JsonReader(text)` |
| `serde_path_to_error::deserialize::<_, T>(&mut reader)` | `$jsonPathRead(reader, readT)`: `Ok` of a `T` read at the reader's place, not to the text's end, or `Err` of `{ path, original }` |
| `T::deserialize(&mut reader)` | `$jsonRead(reader, readT)`, as a hand-written impl's read (ADR 0459) |
| `e.path()`, `e.inner()`, `e.into_inner()` | `e.path`, `e.original`, `e.original` |
| `{}`, `{:?}` of an `Error` or a `Path` | `$displayJsonPathError(e)`, `$debugJsonPathError(e)`, `$displayJsonPath(p)`, `$debugJsonPath(p)` |
| `Box<dyn Error>` of an `Error` | `$jsonPathErrorDyn(..)`'s dictionary |
| an `Error`, a `Path` in a crate's `.d.ts` | `@rust-js/std`'s opaque `JsonPathError`, `JsonPath` (ADR 1149) |

- **The reader keeps the chain; an error copies it as it's made.** A
  read through serde_path_to_error gives the reader a `chain`, the
  segments of where it is, and `$JsonError.chain` the same array; a plain
  `serde_json::from_str` gives none, so each step is one `undefined` test,
  and its JS output is unchanged. The crate records the chain at the
  innermost wrapper an error passes, which is where the reader was as the
  error was made, so `new $JsonError` copies it then. Since an error can
  be made where it isn't yet placed and placed later (`fixPosition`), the
  placed error keeps the copy.
- **Each place pushes its segment for exactly the span the crate's
  wrapper covers.** One owner of the rules, the runtime reader
  (`json_content.js`, `json_reader.js`), beside `$JsonError`:
  - an item: its index, while its value is read; a comma, a `]` or a
    tuple's `invalid length` is the sequence's own;
  - a key: `?` from looking for it until its text is read, then the text,
    until the key is read, `CaptureKey`'s; an integer key its digits and a
    `bool` key its word, as the crate formats them; a float key, or a
    `bool` key serde_json refuses itself, stays `?`;
  - a struct's field name is read as its key, so `deny_unknown_fields`'
    error is at the key, `customer.extra`, while a duplicate or a missing
    field is the struct's, `customer` (the struct reader asks its key for
    its field, `field()`, of each key kind);
  - a value: its key, while it's read; the `:` before it is the object's;
  - a variant: its name's text, while what it holds is read; its name's
    own mistakes, an unknown variant, are the enum's;
  - `Option`, a newtype, a flattened field, an internally tagged or an
    untagged variant's content: no segment, as the crate's wrapper sees
    none of serde's `Content` (ADRs 0079, 0204). What's buffered into a
    `Content` is still read through the tracked reader, so a syntax error
    in it is placed.
- **Its `Error` is serde_json's and the path**, `{ path, original }`, a
  path the segments: a number an index, a string a key, `{ variant }` a
  variant's name, `null` a key that isn't text. `Display` is the crate's:
  the path, `: `, then serde_json's error, the path left out where it's
  none or only `?`; a `Path`'s is `.` of none, `[1]` an index with no dot
  before it. Neither reads the `Formatter`'s options. `Debug` is the
  derived one, `Error { path: Path { segments: [Map { key: .. }, Seq {
  index: .. }, Enum { variant: .. }, Unknown] }, original: Error(..) }`,
  pretty and with `{:x?}`'s index. Into a writer, each piece is the
  crate's `write_str`.
- **Only serde_json's reader of text.** `deserialize` of a `Value`, the
  crate's own `Track` and `Deserializer::new`, `Path::iter` and its
  `Segment`s are refused where they're called; so are serde_json's
  reader's `end`, streams and the byte and io entry points (#1626).
- **The catalog owns the version** (ADR 1185): another version, a fork, or
  one by path or git is its types alone, and a call into it names 0.1.20.

## Alternatives

- **Compile the crate.** Its wrapper wraps calls rust-js never makes.
  Making the derive go through serde's generic machinery is #1552's plan,
  far larger, and every plain read would pay for it.
- **Track always, and let `from_str` drop the path.** Every read would
  pay for a chain no one reads.
- **An API of rust-js's own**, a `from_str_with_path`: refused by the
  one-way rule (idiomatic Rust in).
- **Catch at each place, as the crate's `trigger` does.** The struct
  reader checks a field's name after its key is read, so an unknown
  field's error would be placed at the struct; the copy as the error is
  made needs only the chain to be right at each moment.

## Why

`test/serde-path.test.ts` compiles a program of every rule above and runs
it beside native serde_path_to_error, line for line: items,
keys of text, of numbers past `u32`, of floats and of `bool`s, variants
external and internal, unknown and duplicate fields, a missing `:`, a key
not found, an ignored key's broken value, a `Value`'s inside, a flattened
field, a nested `Vec` in a struct in a `Vec`, a root `Vec`, two reads of
one reader, `into_inner()`, `{:#?}`. A second program shows the `Error`
padded, as `{:x?}` and `{:#x?}`, written into a writer, boxed as a `dyn
Error`, and reads by a type's own `deserialize`. The refusals and the
declarations are held by the same file.

## Consequences

- `$JsonError` has a static `chain`, and a tracked error a `path`.
- `structMap` asks a map's key for its field, `field()`, which each key
  kind has, where it read the key and then looked the field up.
- Mutations: each segment's span (an item's index, a key's `?`, its text,
  a number's or a `bool`'s, a float's none, a variant's, a value's, the
  colon's and an unknown field's place), the error's copy, a placed
  error's path, the `Display`'s dot before an index, `.` and the omitted
  path, the `Debug`'s `Enum`, `{:x?}`'s index, `inner()`, the `dyn`'s
  dictionary, the read of anything but text, and `Path` declared as
  `Error`.
