# 0686. The stack trace hook's type is named, `js_error::PrepareStackTrace<R>`

Status: Accepted. Amends [0648](0648-v8s-stack-trace-api-is-the-node-crates.md),
whose binding spelled the hook's type inline.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. A type's name in
Rust: nothing a program does, nor its JS, changes.

## Context

React's `DefaultPrepareStackTrace`, ported, is a `const` of the hook's
type, as ADR 0648 wrote it:

```rust
js::export_default! {
    pub const DEFAULT_PREPARE_STACK_TRACE: Option<&'static dyn Fn(&JsError, &[&CallSite]) -> String> = None;
}
```

`cargo clippy -- -D warnings` refuses it: `clippy::type_complexity`, "very
complex type used". The port can't name it for itself without a stand-in
for what the node crate binds, and an `#[allow]` for rust-js's sake is
what AGENTS.md rules out. The binding's getter and setter spelled it out
too, each its own copy of one type.

## Decision

**The node crate names the hook's type**, in `js_error`, beside the
getter and setter that take and give it:

```rust
pub type PrepareStackTrace<R> = dyn Fn(&JsError, &[&CallSite]) -> R;

pub fn prepare_stack_trace() -> Option<&'static PrepareStackTrace<Option<&'static Unknown>>>;
pub fn set_prepare_stack_trace<R>(hook: Option<&'static PrepareStackTrace<R>>);
```

A port writes `Option<&'static PrepareStackTrace<String>>`, clean under
clippy's defaults. It's a type alias, so a Rust function given by
reference, `Some(&prepare_stack_trace)`, and a hook read and set back are
as they were, and the JS is the same.

## Why

- **@types/node's shape**: it types the property `prepareStackTrace?:
  ((err: Error, stackTraces: NodeJS.CallSite[]) => any) | undefined`, one
  function type the getter and setter share; Rust, unlike TypeScript,
  asks a long type to have a name.
- **One place for the type**: the getter, the setter and a program's
  `const` name it, none spells its own copy, as `process`'s events name
  each listener's, `type Listener = dyn Fn(&JsError, ..)`.
- **`R` stays the program's**: a hook Rust sets gives any `R`, one JS set
  an `Unknown` (ADR 0648), so the name is generic, as the setter is.

## Alternatives

- **A name of the `Option<&'static ..>` whole**, `type Hook<R> =
  Option<&'static dyn Fn(..) -> R>`: shorter to write, but hides the
  `Option` that is `undefined`, and the reference a Rust function is given
  by, which every other JS function value in the crates shows.
- **`PrepareStackTrace` without its `R`**, of `Option<&'static Unknown>`:
  the getter's, but a Rust hook gives its own text, a `String`, not an
  `Unknown`, as ADR 0648's alternatives say.
- **Leave it, and the port allows the lint**: an `#[allow]` for rust-js's
  sake.

## Consequences

- A program writes the name, as the guide and the tests do. The type
  spelled out is the same type to rustc, which only clippy says is too
  complex.

## What holds it

- `test/node-bindings.test.ts`, "the stack trace hook's type is named,
  clean under clippy": React's `DefaultPrepareStackTrace` and
  `formatOwnerStack` checked by `cargo clippy -- -D warnings`.
- "V8's stack trace hook is saved, set and restored": the `const` of the
  name, compiled and run in Node.
