# 0225. A JS value of unknown shape is a `js::Unknown`

Status: Accepted. Extends [0102](0102-js-and-webapi.md), [0214](0214-untagged-enums.md)
and [0024](0024-web-crate.md).

## Context

Every language that types JS has a value of unknown shape, and a way to
ask it what it is ([research](../research/type-foundations.md#layer-1-the-js-standard-library-and-the-escape-hatch)):
TypeScript's `unknown`, narrowed by `typeof`, and `obj[key]`; Scala.js's
`js.Any` and `js.Dynamic`; ReScript's `unknown`, told by
`Type.Classify.classify`, `JSON.t`, and `@get_index`. rust-js had none. A
binding could declare anything, trusted, cast with `"this"`, or hold an
opaque `&JsObject`, but nothing could hold what `JSON.parse` gives and look
inside it: `webapi` skipped every member WebIDL types `any`,
`Response.json()` among them.

## Decision

**The `js` crate's `Unknown` is a JS value of any shape, never `undefined`
nor `null`, which an `Option<&Unknown>` is `None` of (ADR 0030).
`classify` tells what one is, by `typeof`, and `get` and `set` reach its
properties by name.**

```rust
use js::{Kind, Unknown, classify, json, object};

fn show(value: Option<&Unknown>) -> String {
    match value {
        None => "null".to_string(),
        Some(value) => match classify(value) {
            Kind::String(text) => format!("'{text}'"),
            Kind::Number(n) => n.to_string(),
            Kind::BigInt(n) => format!("{n}n"),
            Kind::Bool(b) => b.to_string(),
            Kind::Array(items) => /* each an Option<&Unknown> */,
            Kind::Object(fields) => /* object::keys(fields), js::get(fields, key) */,
        },
    }
}
js::set(value, "name", "new");
```

```js
function show(value) {
  if (value == null) {
    return "null";
  } else {
    const match = value;
    if (typeof match === "string") {
      // ..
    } else if (Array.isArray(match)) {
      // ..
    } else {
      // Object.keys(match), match[key]
    }
  }
}
value.name = "new";
```

- **`Kind` is an untagged enum** (ADR 0214): `String(&str)`, `Number(f64)`,
  `BigInt(i64)`, `Bool(bool)`, `Array(&[Option<&Unknown>])`, and
  `Object(&Unknown)`, its `otherwise`: an object, a function, a symbol.
  `classify(value)` is a binding whose JS is the value itself, so a `match`
  of it is a `typeof` chain.
- **`get(value, key)` is `value[key]`** and `set(value, key, to)` is
  `value[key] = to`, by two new link-name forms, `"get []"` and `"set []"`,
  as ReScript's `@get_index` and `@set_index`. A key written that's a name
  is `value.name`, as a person writes it.
- **`json::parse(text)`** is `JSON.parse`, a `Result` of what it throws
  (ADR 0035): an `Option<&Unknown>`. `object::keys(value)` is `Object.keys`.
- **What WebIDL types `any`, `webapi` gives as an `Option<&'static Unknown>`**:
  `response::json(r)` is a `Promise<Option<&'static Unknown>>`, and so are
  `MessageEvent.data`, `History.state`, `Window.opener` and
  `AbortSignal.reason`, which were skipped.
- **Its `.d.ts` is TypeScript's `unknown`**: `#[rust_js::types = "unknown"]`.
- **Every step out of the types is written**: `classify`, `get`, a `match`.
  There's no `any` that a value becomes silently, as TypeScript's is, and
  no raw JS, ReScript's `%raw`, which rust-js couldn't read.

## Why

- **It's ReScript's `unknown` and `Classify`, and TypeScript's `unknown`
  with narrowing**, from what rust-js had: an untagged enum is already a
  `typeof` chain, and its `otherwise` variant is "anything else".
- **It's tested**: a compiler test parses JSON, shows each kind it holds,
  walks an object's keys with `get`, sets a property, refuses text that
  isn't JSON, and binds `response.json()`; the JS is a `typeof` chain,
  `match[key]`, `value.name = ..` and `response.json()`; and a declarations
  test's function of `Unknown`s is declared `unknown`.

## Not yet

- **A WebIDL parameter typed `any`**, seven of them, `postMessage`'s say:
  still skipped, as what Rust gives one is open.
- **A typed JSON value**, ReScript's `JSON.t`: an object's fields would need
  a dictionary type, which the `js` crate doesn't have.
- **`Option::flatten`**, which rust-js doesn't take yet, would make
  `json::parse(text).ok().flatten()` one step.

## Costs

- The `js` crate's next release has `Unknown`, and `webapi`'s needs it: the
  two are released together.
