# 1003. A compiler's build is part of its identity

Status: Accepted. Extends the compiler identity of manifests (ADR 0100)
and of `rustc -vV` through rust-js, which Cargo hashes into each crate's
fingerprint.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; what one compiler
decided is never read by another build of it.

## Context

React's reconciler landing agent saw ADR 0852's refusal, "this crate
counts an `Rc<ReactContext<..>>` .. but `react_isomorphic` .. counts none",
after rust-js's main checkout moved on: the same sources built cleanly in
a fresh copy, and touching `shared`'s and `react`'s roots, which rebuilt
them, made it go away. Their libraries' manifests had been written by
another build of the compiler.

A compiler's identity was its version, its toolchain and its manifests'
ABI: `rust-js: 0.0.4, Rust 1.99.0, ABI 1`. Every build of one version
said the same. So a library's manifest, whose facts are what that build
decided (which `Rc`s it counts, whether its `dyn Fn`s drop, which drops
and copies its exports take), was read as this build's by any other build
of the version. And Cargo, whose fingerprint of a crate hashes what `rustc
-vV` says, had a crate another checkout's or worktree's compiler built as
done: the compiler a crate's dep-info names is that other one, whose file
hadn't changed. The library's contract version (`library::VERSION`, 10)
is bumped by hand where its fields change shape, not where what fills them
does.

## Decision

**A compiler's identity includes its build: a hash of what it's made of,
its sources (with the runtime helpers and the std manifest they build in),
its toolchain file, its build script, and its package's manifest and lock
file** (`build.rs`'s `RUST_JS_BUILD`, `manifest::BUILD`).

- **Manifests say it**, `"compiler": { .., "build": "52c100506ffb5402" }`,
  and a consumer reads a library's only where it's its own build's, as
  before: "incompatible dependency compiler identity". A manifest of a
  compiler before this one has none, and is refused the same way.
- **Cargo is told it**: `rust-js: 0.0.4, Rust 1.99.0, ABI 1, build
  52c100506ffb5402`, so a crate another build compiled is built again.
- **`@rust-js/std`'s manifest says none** (`Manifest::of_no_build`): it's
  committed, built into each build of the compiler, which doesn't check
  its identity.
- **The browser's compiler**, `wasm/`, builds with the same script, its
  own lock file among what it hashes; its rpath, the toolchain's, is the
  native compiler's alone.

## Why

- **One source of a fact, its version checked**: what a library's
  manifest says is a cache of what its compiler decided, and a cache is
  keyed by what made it (AGENTS.md). A build that decides otherwise reads
  none of another's.
- **The hash of the inputs, not of the binary**: the same sources give the
  same identity on every machine, and asking it costs nothing at run time;
  hashing a 100 MB compiler at each crate's compile would.

## Alternatives

- **Bump `library::VERSION` with each change to a fact**: by hand, and
  forgotten where a field's meaning, not its shape, changes, as here.
- **The binary's modification time**: Cargo already reads it, of the one
  compiler a crate's dep-info names; another checkout's compiler is another
  file, as old as it is.

## Consequences

- Each change to the compiler's sources rebuilds every crate of a Cargo
  build that compiler runs, as a change to the binary already did of the
  same checkout's.
- Libraries built by a compiler before this one are refused until rebuilt.

## What holds it

- `test/cargo-workspace.test.ts`, "Cargo is told which rust-js it runs":
  the build in `rustc -vV`'s line, as `--version-json` says it, and in
  Cargo's `.rustc_info.json`.
- `test/cargo-link.test.ts`, "a real Cargo dependency links": a library's
  manifest of another build, or of none, is refused.
- `test/manifest.test.ts`: the host reads a build of sixteen hex digits.
- `test/std-package.test.ts`: std's manifest says no build.
- Mutations `cargo-version-without-build` (`cargo.rs`),
  `manifest-build-unread` (`manifest.rs`) and `std-manifest-of-a-build`
  (`output.rs`).
