# 1106. A module exports nothing named `then`

Status: Accepted (the user's decision, 2026-11-09, #1068: refuse, of the
options refuse, rename and refuse only a library's).

Case ([0262](0262-when-rust-and-js-disagree.md)): D, Rust's behaviour
can't be kept, so it's a compile error. Rust's module of a function `then`
is a module; JS's module that exports `then` is a thenable, which `import()`
resolves by calling it.

## Context

The object model's net (ADR 1105) loads each library a case's items are
moved into with `import()`. Of `match_of_what_a_match_made`, whose
`fn then(chunk, resolve, reject)` the `library` rewrite made `pub`, the
load crashed: an ES module namespace that has a `then` export is a
thenable, so `await import("./lib.js")` called `then(resolve, reject)` as
the library's function, which read `chunk.state` of a resolver function.
A host that loads a module lazily, a bundler's lazy chunk, Vite's
`import()`, does the same: the import resolves to whatever `then` gives,
or never resolves, not to the module.

## Decision

**What a module of the crate's would export by the JS name `then` is a
compile error, saying to give it another JS name**: a `pub fn`, `static`
or `thread_local!` of a module, by its name or its `#[rust_js::name]`
(`analysis/naming.rs`'s `name_items`, where each export's name is
claimed); a `pub use` of the module's own item as `then`
(`default_exports::aliases`); and a `pub use` of another's
(`reexports::reexports`). One owner says it, `bindings::then_export_refused`,
beside `exported_by_name`:

```text
error: rust-js: `then` is exported as `then` in JS, which makes its module a thenable: `import()` of it, a host's or a bundler's lazy chunk, calls it as `then(resolve, reject)`; give it another JS name, `#[rust_js::name = ".."]` (#1068, ADR 1106)
```

```rust
#[cfg_attr(rust_js, rust_js::name = "thenChunk")]
pub fn then(chunk: &Chunk) {}
```

A method named `then`, its type's, a property of its class or object, and
a function the module keeps to itself are no export of the module, and
compile as before.

## Alternatives

- **Renamed, `then` to `then$`**: nothing refused, but the name
  TypeScript sees is one nobody wrote, and an importer of `then` finds no
  such export.
- **Refused of a library's modules alone**: a bundler may load a program's
  own module lazily too, as a chunk.

## Consequences

- The corpus case `then_export_refused` and the diagnostics of each way a
  module exports a name hold it; mutations `then-export-allowed`,
  `then-alias-allowed` and `then-reexport-allowed`. The guide's limitations say it.
- `match_of_what_a_match_made`'s `then` is private, so the case compiles as
  before; its `library` rewrite, which makes it `pub`, is refused, which
  the metamorphic and object-model nets take as a refusal.
- A value that has a callable `then`, a class's method's, that an `async
  fn` returns or `Promise.resolve` is given, is a thenable too: probed
  apart, against native in the async table.
