# 0583. A global some hosts lack is an `Option`

Status: Accepted. Extends [0021](0021-js-interop.md)'s globals and
[0030](0030-option.md)'s `None`.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A global that's
there is read as it was; one that isn't is `None`, where JS threw.

## Context

React's scheduler picks how to run its work by what the host has:

```js
const localSetImmediate =
  typeof setImmediate !== 'undefined' ? setImmediate : null; // IE and Node.js + jsdom
..
} else if (typeof MessageChannel !== 'undefined') {
```

A binding's global was its name (ADR 0021), so a Rust program had no way
to ask whether the host has it: reading one it lacks throws a
`ReferenceError`. Scala.js writes `typeof x` only of `js.typeOf` given
`js.Dynamic.global.x` itself, and throws where a helper is between;
ReScript's `%external`, which gave an option, was removed.

## Decision

**A global some hosts lack is declared an `Option`, and read as
`undefined` where the host hasn't it**:

```rust
unsafe extern "Rust" {
    #[link_name = "setImmediate"]
    pub safe static SET_IMMEDIATE: Option<&'static SetImmediate>;
}

if let Some(set_immediate) = SET_IMMEDIATE { .. }
```

```js
const tmp = typeof setImmediate !== "undefined" ? setImmediate : undefined;
```

A global every host the binding is for has stays its type, read as its
name: `webapi::performance`. Each global has one declaration, of one of
the two.

`js::type_of` stays `typeof value` as text, for a message; what a value
is, to branch on, is `js::classify`'s.

## Why

- **The question is a type's**: whether a global may be missing is
  `Option`, which Rust makes every reader answer; it's asked where the
  global is read, whatever is between.
- **It's the JS people write**: `typeof x !== "undefined"`, the one form
  that reads a missing global without throwing.

## Alternatives

- **A `js::type_of_global("setImmediate")`**: a name as text, unchecked,
  and a second way to read a global. Rejected.
- **`globalThis.setImmediate`**: reads a missing one too, but isn't what
  upstream writes, and a `let` global isn't on it. Rejected.
- **A `#[rust_js::optional]` attribute**: what the `Option` already says.
  Rejected.

## Consequences

- No binding rust-js has changes; `setImmediate` and `MessageChannel`
  join as `Option`s where they're bound.

## What holds it

- `test/lowering.test.ts`, "an Option global binding is undefined where
  the host lacks it": one there, called; one missing, `None`.
- Mutations: the global read unguarded.
