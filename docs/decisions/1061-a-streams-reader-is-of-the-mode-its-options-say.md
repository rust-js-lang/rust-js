# 1061. A stream's reader is of the mode its options say, as TypeScript's overloads type it

Status: Accepted (rd506, P2, 2026-11-02). Amends
[0102](0102-js-and-webapi.md) (webapi's members are WebIDL's, typed as
TypeScript's DOM types them) and applies
[0442](0442-a-web-api-that-may-throw-is-a-result.md) (what throws for
what a program can't check first is a `Result`).

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The JS is the call
upstream writes; only the Rust types of its result change.

## Context

React's Flight reply client reads a byte stream as bytes and any other
as values:

```js
let binaryReader;
try {
  binaryReader = stream.getReader({mode: 'byob'});
} catch (x) {
  return serializeReader(stream.getReader());
}
return serializeBinaryReader(binaryReader);
```

WebIDL declares one operation, `ReadableStreamReader getReader(optional
ReadableStreamGetReaderOptions options = {})`, its result a union of the
two readers, so webapi's `get_reader(Option<ReadableStreamGetReaderOptions>)`
gave the union and no `Result`. The port matched the union, an
`instanceof` test and an `unreachable!()` arm upstream doesn't have, and
caught the `TypeError` with `catch_unwind`. TypeScript's DOM types the
call by its argument:

```ts
getReader(options: { mode: "byob" }): ReadableStreamBYOBReader;
getReader(): ReadableStreamDefaultReader<R>;
getReader(options?: ReadableStreamGetReaderOptions): ReadableStreamReader<R>;
```

## Decision

**`getReader` is two bindings, as TypeScript's overloads are, each of the
reader its arguments give** (`EXTRA.ReadableStream` in
`webapi/generate.ts`):

| Rust | JS | Gives |
|---|---|---|
| `stream.get_reader()` | `stream.getReader()` | `&ReadableStreamDefaultReader` |
| `stream.get_reader_with_options(ReadableStreamGetByobReaderOptions { mode: Byob })` | `stream.getReader({ mode: "byob" })` | `Result<&ReadableStreamBYOBReader, &JsError>` |
| `stream.get_reader_with_options(ReadableStreamGetReaderOptions { mode })` | `stream.getReader({ mode })` | `Result<ReadableStreamReader, &JsError>` |

```rust
let binary_reader = match stream.get_reader_with_options(ReadableStreamGetByobReaderOptions { mode: Byob }) {
    Ok(binary_reader) => binary_reader,
    Err(_) => return serialize_reader(stream.get_reader()),
};
serialize_binary_reader(binary_reader)
```
```js
let binaryReader;
try {
  binaryReader = stream.getReader({ mode: "byob" });
} catch {
  return serializeReader(stream.getReader());
}
return serializeBinaryReader(binaryReader);
```

- **`{ mode: "byob" }` is a type**, `ReadableStreamGetByobReaderOptions`,
  whose one field is `Byob`, a unit struct whose value is the string
  `"byob"`, as an event's name is its type's (ADR 0223): TypeScript's
  literal, which the first overload turns on. A sealed trait,
  `ReaderOptions`, says which reader each options type gives, so the one
  method types each overload's result.
- **The options' form is a `Result`**: a BYOB reader of a stream that
  isn't a byte stream throws a `TypeError`, which no program can check
  first, since no member says what kind a stream is (ADR 0442). Locking a
  stream that's locked throws too, but `stream.locked()` says so first: a
  misuse, a panic, as `get_reader()`'s.
- **Two functions, not one of an `Option`**: they differ in their result,
  which ADR 0443 keeps apart (its rule 5); `get_reader(None)` would need
  the options' type named to say which reader it is. The second is named
  as the generator names a later overload, after the argument it adds.

## Alternatives

- **Keep WebIDL's union**: the port's `instanceof` test and
  `unreachable!()` arm, which upstream doesn't write, for every BYOB
  reader.
- **A BYOB reader of `ReadableStreamGetReaderOptions { mode: Some("byob")
  }`**: the dictionary's mode is any text, `None` too, whose reader is a
  default one, so its type would be wrong of one.
- **`new ReadableStreamBYOBReader(stream)`**, which webapi binds: the same
  reader and the same `TypeError`, but not the JS upstream writes.

## Consequences

- Nothing outside the Flight draft called `get_reader`; its `catch_unwind`
  and both `unreachable!()`s go.
- Held by `test/webapi-objects.test.ts` ("a stream's reader is of the mode
  its options say, a BYOB one a Result"): the JS above, a byte stream's
  BYOB reader and a default stream's fallback, run, and the general
  options' union of either mode.
