# 0360. The app's crate is a program: `RUST_JS_APP`, recorded for Cargo

Status: Accepted. Amends [0100](0100-separate-crates.md) and
[0101](0101-cargo-workspace-wrapper.md).

## Context

Cargo builds each crate with `--library`, the app's too (ADR 0101): a crate
built as the app would be fresh when another build used it, and Cargo
can't say which crate it was asked for, as it drops `CARGO_PRIMARY_PACKAGE`
from what it tracks. A library assumes another crate may use what it
exports (ADR 0100), so what only a library can't do the app couldn't do
either: of the corpus's programs, 10 compiled alone were refused built by
Cargo, seven of them a counted `Rc` "another crate may share". A Next.js
page is such an app.

## Decision

**The tooling names the app, `RUST_JS_APP`: the package it builds, or its
manifest's own. rust-js compiles that package as a program, and records
`RUST_JS_APP` in the dep-info, as rustc records what `env!` reads, so
Cargo builds a crate again when it changes.**

- `checkCargo` sets it to the package it's asked for, or the manifest's
  package; the editor's check the same, so its errors are the build's. A
  workspace's root without a package names none: every crate a library.
- In one build, nothing uses the app: Cargo builds it and its dependencies.
  A build of another package that uses it has another `RUST_JS_APP`, and
  Cargo builds the app again, a library.
- Its manifest has no `library`; the tooling names its crate as Cargo does.
- A Cargo build without the tooling, no `RUST_JS_APP`, is what it was.

## Consequences

- The workspace test builds the app, a crate that uses it, the app again,
  and the whole workspace: program, library, program, library.
- The react.dev port: one line, `formatStr(consoleData.data)`, as react.dev
  writes it, where a library copied the array.

## Amendment: the corpus as libraries

Each corpus program is compiled as a library too and run against native
Rust. What a library refuses that a program doesn't is its directive,
`//@ library-refused: <text>`, asserted, and an error once it compiles:
seven counted `Rc`s another crate may share (ADR 0320), and two values
with destructors. A trait's default constant a library computed where no
consumer reads it, an error rustc never gave, is now its initializer
(ADR 0176).
