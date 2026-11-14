# 1158. A class that extends `Error` is given its `message` and `cause` by its fields

Status: Accepted (2026-11-15, #1808's first step, the form the lead
approved). Extends [1041](1041-a-class-that-implements-a-bindings-class-extends-it.md):
a binding's class trait may name a global class, not only a module's
export.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is preserved. `Error`'s constructor makes `message` and
`cause` own properties that aren't enumerable; `this.message = ..` would
make enumerable ones, which `Object.keys` and `JSON.stringify` list, so
they're given through `super`. The order Rust evaluates a struct
literal's fields in is kept: a field written before them whose value runs
code is held first.

## Context

vercel-labs/vgpu's SDK wraps native pipeline failures in its own error
class (`packages/core/src/errors.ts`):

```ts
export class VGPUError extends Error {
  readonly code: string;
  readonly severity: VGPUErrorSeverity;
  ..
  constructor(data: VGPUErrorData) {
    super(data.message, { cause: data.cause });
    this.name = "VGPUError";
    this.code = data.code;
    this.severity = data.severity ?? "error";
    ..
  }
}
```

Its port (Codex's vgpu-in-rust) had no form for it: `js::Error` is an
opaque binding struct, and a `#[rust_js::class]` extends a binding's class
only by its class trait (ADR 1041), which the js crate didn't have for
`Error`, and only of a class a module exports.

## Decision

**`js::ErrorClass` is `Error`'s class trait**, `#[rust_js::types =
"Error"]`, named as ADR 1041's precedent names react's: after the JS
class, `react::Component` for `React.Component`, but `ErrorClass`, as
`js::Error` is the struct already. A `#[rust_js::class]` struct that
implements it is an ES class, `class VGPUError extends Error`, named by
the global, imported from nowhere (`classes::BaseClass::Global`).

**What `Error`'s constructor sets is the struct's fields of its names**
(`classes/errors.rs`):

```rust
#[cfg_attr(rust_js, rust_js::class, rust_js::name = "VGPUError")]
pub struct VgpuError {
    message: String,
    cause: Option<&'static Unknown>,
    name: &'static str,
    pub code: String,
    pub severity: &'static str,
}

impl ErrorClass for VgpuError {}

#[cfg_attr(rust_js, rust_js::class)]
impl VgpuError {
    pub fn new(data: VgpuErrorData) -> Self {
        Self {
            message: data.message,
            cause: data.cause,
            name: "VGPUError",
            code: data.code,
            severity: data.severity.unwrap_or("error"),
        }
    }
}
```

```js
export class VGPUError extends Error {
  constructor(data) {
    super(data.message, { cause: data.cause });
    this.name = "VGPUError";
    this.code = data.code;
    this.severity = data.severity ?? "error";
  }
}
```

- **`message`**, a field of text, `String` or `&str`, is `super`'s first
  argument; of another type it's refused, saying so.
- **`cause`** is its options', `{ cause }`, where the struct has the
  field, its `None` `undefined`, there as Rust's field is; of no `cause`
  field, `super(message)`, as `new Error(message)` makes none. No field
  of either, `super()`.
- **Every other field is set after**, `this.x = ..`, in Rust's order,
  `name` too, as upstream sets its own. One written before `message` or
  `cause` whose value does something is held first, `const code =
  next();`, and set after `super`.
- **It writes its constructor**, `fn new(..)` of any parameters, which
  JS's `new` calls: one that writes none is refused, as `Error`
  constructs no class of its own, where React constructs a component.
- **What `Error` has that its constructor doesn't set is the trait's**,
  as `props` is `Component`'s: `stack()`, `get stack`, `None` where the
  host keeps none; and `error()`, the value as the `js::Error` it is, to
  throw or give as one, an identity binding (`link_name = "this"`). Not a
  `Deref`: no trait gives a crate's struct one.
- **Its declaration extends `Error`**, `export declare class VGPUError
  extends Error { constructor(data: VgpuErrorData); message: string;
  cause?: unknown; .. }`, which tsc checks against ES2022's `Error`.

**Fields by name, not an attribute**: `message` and `cause` are JS's own
names of what `Error`'s constructor sets, so the struct says them as its
instance has them, and rustc checks each is written where the struct is
made. ADR 1041 rejected an attribute for what a class extends; one for
which field is the message would be the same unchecked string.

## Why

- **It's upstream's JS**, line by line.
- **JS sees a real `Error`**: `instanceof Error`, a stack of
  `VGPUError: ..`, `String(e)`, `cause` the value given, and the same
  own properties, enumerable or not, as upstream's own class, which the
  test runs beside it.
- **It's Rust's own struct**, its fields typed and checked, its
  constructor a `fn new`.

## Alternatives

- **`this.message = ..` after `super()`**: the property is enumerable, so
  `JSON.stringify(e)` and `Object.keys(e)` differ from upstream's.
- **A call written first, `js::error::init(message, options)`**: a Rust
  statement that does nothing in Rust, and that a constructor could
  forget or write twice.
- **`impl std::error::Error` as the mark**: `Display` is computed where
  JS's `message` is set once; it doesn't say what the stack or the cause
  is, and every crate's own error types would become classes.

## What's left: a class that extends a crate's class (step 2)

upstream's `class ValidationError extends VGPUError` extends a class of
its own crate, which no form says yet, for any class: its own proposal.
Nothing here changes for it: `BaseClass` gains the crate's class, and
`extends_error` asks only of the class's own impl of `js::ErrorClass`.

## Consequences

- `test/error-classes.test.ts`: the class beside upstream's, the same
  keys, JSON, own properties and descriptors, `instanceof`, the
  prototype chain, the stack, `String(e)`, cause identity; one of no
  `cause` with none; one thrown and caught; the order held of a field
  before `message`; the declaration, type-checked; and the refusals of a
  `message` that isn't text and of no constructor.
- Guide: interop/exceptions.md, "A class that extends `Error`", and a
  pointer from interop/export-to-javascript.md.
- Mutations, of `classes/errors.rs`, `classes/subclasses.rs` and
  `classes.rs`: extends-error-unrecognized, error-message-set-after,
  error-cause-of-no-field-given, error-cause-field-unset-left-out,
  error-field-before-message-unheld, error-message-any-type,
  error-class-unconstructed-allowed, base-class-global-unread.
