# 1056. A stream's general source is TypeScript's `UnderlyingSource`

Status: Accepted (rd403, P1, 2026-11-02). Amends
[0102](0102-js-and-webapi.md) (a `ReadableStream`'s source is
TypeScript's, rd452).

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The binding says what
JS's constructor already reads. Nothing runs differently.

## Context

React's Flight client starts a stream of either kind, as its chunk
says:

```js
let controller: ReadableStreamController = null as any;
const stream = new ReadableStream({
  type: type,
  start(c) {
    controller = c;
  },
});
```

Here `type` is `'bytes'` or `undefined`, and `controller.enqueue(value)`,
`controller.close()` and `controller.error(e)` are then called of
whichever controller the stream made. ADR 0102 (rd452) bound TypeScript's
two sources: an `UnderlyingByteSource`, whose `type` is `"bytes"`, and an
`UnderlyingDefaultSource`, which has no `type`. Neither can say "the kind
the program picks when it runs". The port declared its own
`UnderlyingSource` struct as a stand-in, which AGENTS.md forbids.

## Decision

**webapi binds TypeScript's general `UnderlyingSource`: `r#type:
Option<&str>`, `"bytes"` or `None`, `auto_allocate_chunk_size`, and
`start`, `pull` and `cancel`, each lent. `start` and `pull` are given a
`&'static ReadableStreamController`, TypeScript's
`ReadableStreamController<any>`: either controller, read by what the two
have alike, `desired_size()`, `close()`, `enqueue(chunk)` and
`error(e)`.** `readable_stream::new` takes it beside the other two
(`IntoUnderlyingSource`). `webapi/generate.ts` writes it by hand with the
others (`SOURCES_BY_HAND`), as no WebIDL declares it.

```rust
let stream = readable_stream::new(
    Some(&UnderlyingSource {
        r#type,
        start: Some(&|c| controller.set(Some(c))),
        ..Default::default()
    }),
    None,
);
```

```js
const stream = new ReadableStream({
  type,
  start(c) {
    controller = c;
  },
});
```

- **`enqueue` takes any value**, the default controller's chunk, as
  upstream gives either controller its `T`. A byte stream's controller
  throws a `TypeError` of what isn't an `ArrayBufferView`, as JS's does.
  The test holds this.
- **The controller is one type, not a union** of the two controllers. A
  union would make each call test which controller it is, which upstream
  doesn't write. The user decided against that for rd452.

## Alternatives

- **WebIDL's `ReadableStreamController` union, untagged.** Rejected:
  `enqueue` would need a `match` and an `instanceof` test (above).
- **`enqueue` of an `ArrayBufferView` only**, the two signatures' meet as
  TypeScript computes it for `ReadableStreamController<any>`. Rejected: a
  stream of values could take none, and upstream's Flow type takes its `T`.
  JS's own check still refuses a wrong chunk where it matters.

## Consequences

- The Flight client drops its `UnderlyingSource` struct for webapi's, and
  types its `controller` `ReadableStreamController`.
- Held by `test/webapi-objects.test.ts` ("a stream of either kind reads
  from its general source, its controller either's"). It checks the JS,
  the values a default stream reads, the bytes a byte stream reads, the
  byte controller's `TypeError` and the `.d.ts`.
