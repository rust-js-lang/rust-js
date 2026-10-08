# 0273. A `#[path]` module's JS is beside its file

Status: Accepted. Extends [0019](0019-one-js-file-per-module.md) and
[0101](0101-cargo-workspace-wrapper.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A Next.js page's file is named by its route, `pages/errors/[errorCode].tsx`,
which no Rust module can be named: react.dev's port has it as

```rust
pub mod pages {
    pub mod errors {
        #[path = "[errorCode].rs"]
        pub mod errorCode;
    }
}
```

A module's JS went where its module path says, `pages/errors/errorCode.jsx`,
which Next.js doesn't take as the route, and a Cargo build in source wrote
it there too.

## Decision

**A module with a `#[path]` has its JS beside the file it names**:
`pages/errors/[errorCode].jsx`, and below the output's directory as the
file is below the root's, `../pages/codes/[code].jsx` of `app/page.rs`'s
crate. What it imports, and what imports it, is seen from there: the
crate's other modules, and a relative link name, which is the root's
directory's (ADR 0028). A Cargo build in source writes it beside its file,
as the compiler placed it.

## Why

- **Rust's layout is where the file is**: `#[path]` says where, and the JS
  beside its Rust is what ADR 0101 writes.
- **It's what a framework that routes by files needs**, Next.js's pages.
- **It's tested**: a compiler test has a root and a `#[path]` module import
  each other and a file beside the root, and the Next.js example builds a
  page of `pages/codes/[code].rs`; mutations place it by its module path.

## Since

- **One outside the root's directory keeps its module path's place in the
  output**, `../shared/model.rs` of `client/lib.rs`'s crate: its JS stays in
  the output's directory, where a direct compile writes, not beside a file
  outside it. A Cargo build in source writes it beside its file still, as
  the manifest says it's a `#[path]` module's, `located`.
