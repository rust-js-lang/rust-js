# 0440. A binding needs no `#[allow]`

Status: Accepted. Extends [0039](0039-generic-bindings.md) and
[0439](0439-a-component-is-named-as-jsx-names-it.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): a lint's warning
changes nothing a program does; and "Idiomatic Rust in" (AGENTS.md).

## Context

A binding is a function JS's function is called for, its body never run,
`#[rust_js::link_name = "setTimeout"] fn set_timeout(..) { unreachable!() }`.
rustc saw two things there that aren't so, and the playground and the
examples wrote about sixty `#[allow]`s for them:

- **Its parameters unread**, by the body that never runs:
  `#[allow(unused_variables)]` on each binding.
- **The fields of a struct given one never read**, which JS reads:
  `StateConfig { doc, extensions }` given CodeMirror's
  `EditorState.create`, the WASI shim's `{ readonly: true }`, given as a
  `&dyn Any`: `#[allow(dead_code)]` on each.

And some apps bound what the crates bind: `setTimeout` and `new Promise`
(countdown), `new KeyboardEvent` (todo), stand-ins AGENTS.md rules out.
`#[allow(clashing_extern_declarations)]` on `extern` blocks was needed by
no binding rust-js takes.

## Decision

**rust-js tells rustc what a binding is**, so no program writes an
`#[allow]` for one:

- **A binding's parameters**: the `jsx!` pass, which allows a component's
  name (ADR 0439), allows `unused_variables` on each function it finds
  `#[rust_js::link_name]` on.
- **What JS reads**: rustc's liveness, the query its dead-code lint asks
  (`live_symbols_and_ignored_derived_traits`), is rust-js's around
  rustc's: each field of a struct of the crate's in the type of a value a
  body gives a binding, as it is before it's coerced, and of a struct in
  such a field, is live (`lower::liveness`). A struct given only Rust's
  functions, or none, is flagged as it was.
- **The apps** drop their `#[allow]`s, the countdown and todo examples
  use the js and webapi crates' `set_timeout`, `promise::new` and
  `keyboard_event::new_with_event_init_dict`, and imports only their
  tests use are their tests'. The playground compiles with no warning,
  which its stable-Rust test holds.

## Why

- **The Rust is the Rust a server is written in**, no lint quieted for
  rust-js's sake, guessable from one example.
- **The answer is rust-js's**, which knows what a binding is: the one
  place, `is_binding`, says so to the liveness, and the `link_name`
  attribute to the pass.

## Consequences

- A plain `cargo check`, which runs neither, still warns, as it warns of
  what only JSX uses (ADR 0439).
- todomvc keeps its own `localStorage` and `JSON` bindings: webapi's
  `localStorage` throws as a panic, not an `Err`, where a sandboxed frame
  forbids it, and the playground has no serde to write JSON with. Each is
  a gap of its own.
- The countdown's JS is `setTimeout(() => { resolve(); }, ms)` where its
  own binding's was `setTimeout(resolve, ms)`: `js`'s `Promise` `resolve`
  takes the value, here a `()`, which is no argument (ADR 0441); the
  arrow stays, as what `setTimeout` gives its callback isn't Rust's to say.

## Amendment: each body with its own results

The liveness walked into a body in a body, a block given as a const
argument, `ConstU32<{ identity::<3>() }>`, with its owner's type-check
results, which have no type for it: rustc's `const_fn_in_generics.rs`
crashed. Each body owner is walked on its own, with its own results; the
binding test holds a call inside a const argument.
