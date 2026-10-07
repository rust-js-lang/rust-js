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
  `AbortSignal.reason`, which were skipped. **And one it takes is of any
  type**, as JS has it: a generic parameter of its own, `push_state<D>(history,
  data: D, ..)`, so `push_state(history, Saved { page }, "")` is
  `history.pushState({ page }, "")`: `postMessage`, `structuredClone`,
  `reportError`, a stream's `cancel`. (Amended: they were skipped.) A
  dictionary's field typed `any` is still left out, as a struct's field
  can't be generic.
- **One the browser copies takes only what it copies as it is**,
  `js::StructuredClone`: `postMessage`'s message, `pushState`'s and
  `replaceState`'s data, and `structuredClone`'s value, as the generator
  lists them, which WebIDL doesn't say. Numbers, strings and `bool`s are,
  arrays, tuples, `Vec`s and `Box`es of them, an `Option` of what's
  `js::Defined`, never nullish, so never `Some(None)`'s box, `Json`,
  `Dict`, `Unknown`, and what WebIDL marks `[Serializable]`, a `Blob`. A
  struct is one by its `unsafe impl`, which vouches its fields are. A
  closure, which the browser can't copy, a `Window`, and `Some(None)`,
  which would arrive as rust-js's `{ $someNone }`, are rustc's errors.
  `reportError`'s error and `cancel`'s reason, which nothing copies, take
  anything still, as JS does. (Amended: they took anything.)
- **JSON is a typed value too, `js::Json`**, as ReScript's `JSON.t` is: an
  untagged enum of `String`, `Number`, `Bool`, `Array(&[Option<Json>])`
  and, otherwise, `Object(&Dict<Option<Json>>)`, a JSON `null` the `None`
  of the `Option` that holds it. `Json::parse(text)` is `JSON.parse`, and
  `Json::stringify(&value)` `JSON.stringify`. (Amended.)
- **A dictionary is `js::Dict<T>`**, ReScript's `dict`, TypeScript's
  `Record<string, T>`: a plain object. `dict::entries`, `keys`,
  `from_entries` and `set` are `Object`'s; `dict::get(d, key)` is the
  runtime's `$dictGet`, `Object.hasOwn(d, key) ? $some(d[key]) : undefined`,
  so a key that isn't its own, `toString`, is `None`, and of a
  `Dict<Option<_>>` a JSON `null` is `Some(None)`, as `d[key]` alone
  couldn't tell. A binding of a runtime helper is imported with the helpers.
- **Its `.d.ts` is TypeScript's `unknown`**: `#[rust_js::types = "unknown"]`.
  A `Dict<T>`'s is `{ [key: string]: T }`, and a `Json`'s the union of its
  payloads, declared in the module that names it, as another crate's
  untagged enum is, named, so it can be recursive:
  `type Json = string | number | boolean | (Json | null | undefined)[] | { [key: string]: Json | null | undefined }`.
  (Amended: they were `any`.)
- **What's never nullish is an `Unknown` too**, `js::unknown(text)`, as any
  value is TypeScript's `unknown`: a `Defined` value, the value itself.
  react.dev's ConsoleBlock's message is its text or an element's
  `props.children`, either an `Unknown`. **And `js::string(value)` is
  `String(value)`**, any value as text as JS makes it, `"undefined"` of
  `None`, as `result += child.props.children` does. (Amended.)
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

## Costs

- The `js` crate's next release has `Unknown`, and `webapi`'s needs it: the
  two are released together.
