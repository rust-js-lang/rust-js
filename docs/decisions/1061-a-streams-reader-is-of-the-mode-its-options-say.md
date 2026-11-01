# 1061. A stream's reader is of the mode its options say, as TypeScript's overloads type it

Status: Accepted (rd506, P2, 2026-11-02). Amends
[0102](0102-js-and-webapi.md) (webapi's members are WebIDL's, typed as
TypeScript's DOM types them) and applies
[0442](0442-a-web-api-that-may-throw-is-a-result.md) (what throws for
what a program can't check first is a `Result`). Amended (2026-11-02,
rd505): a form's `append` and `set` take text or a Blob, as TypeScript's
first overload does.

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

## Amendment: a form's `append` and `set` take text or a Blob (2026-11-02, rd505, rd397)

React's Flight reply client copies a form's entries into the one it
sends, each as it is, a string or a `File`:

```js
value.forEach((originalValue, originalKey) => {
  data.append(prefix + originalKey, originalValue);
});
```

WebIDL declares `append(name, USVString value)` and `append(name, Blob
blobValue, optional USVString filename)`, so webapi's `append` took text
and `append_with_blob` a Blob, and the port matched each entry,
`FormDataEntryValue::Str(text) => data.append(.., text), File(file) =>
data.append_with_blob(.., file, None)`, a test upstream doesn't make.
TypeScript's first overload takes either: `append(name: string, value:
string | Blob): void`.

**`append` and `set` take a value as TypeScript's first overload does,
`impl IntoStrOrBlob`, and the Blob's form is the one that gives a file
name**, `append_with_blob(name, blob_value, filename: &str)`:
`PARAM_TYPES` in `webapi/generate.ts`, now each parameter's whole type
by its interface's member and its name, the constructor's as before.

```rust
value.for_each(Box::new(move |original_value, original_key, _| {
    data.append(&(prefix.clone() + original_key), original_value)
}));
```
```js
value.forEach((originalValue, originalKey) => {
  data.append(prefix + originalKey, originalValue);
});
```

- A form's entry, `FormDataEntryValue`, is a `string | Blob` already
  (`IntoStrOrBlob for FormDataEntryValue`), so it's given as it is, as
  upstream gives it.
- The file name required: `data.append("blob", blob)` is `append`'s, one
  way, where `append_with_blob(.., None)` was another.
- Case N: the JS call is the one upstream writes.

Held by `test/webapi-objects.test.ts` ("a form's append takes text or a
Blob, an entry as it is"), run in Node.
