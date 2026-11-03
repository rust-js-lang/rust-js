# 1068. A workspace is built whole, its programs the workspace's

Status: Accepted. Amends [0360](0360-the-app-is-a-program.md) and
[0412](0412-rust-js-build-is-its-own-process.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): none; this decides how
a workspace's crates are built and which of them is a program, and each
crate's JS is what it was.

## Context

React's port regenerated its JS one crate at a time, `rust-js build
--package <name>` for each of its 18 crates, and took 30 minutes on the
Mac (1,827 s, 2026-11-02), one core busy of 12 (rd662). Cargo keeps one
target directory for all of them, so each dependency should have been
checked once. It was checked in every build that used it:

- **`RUST_JS_APP` was the package asked for.** Every crate rust-js
  compiles records it in its dep-info, so Cargo builds it again when it
  changes (ADR 0360). A build of `react-is` after one of `shared` checked
  `shared` again, and one of `shared` after that again: its `.rmeta`
  written at 18:45:37, :40 and :43, each a library, the same bytes. Built
  twice in a row, the same package was done in 0.65 s.
- **A workspace couldn't be built whole.** `rust-js build` at a
  workspace's root stopped, "name the app's package". Asked for the whole
  workspace, `checkCargo` made every crate a library, so a crate nothing
  uses was written one way built alone and another built with the rest: a
  workspace build wasn't what each package's build writes.

Since ADR 0360's amendment a crate another uses is a library whichever
package is built. Which crates are programs is then the workspace's alone:
those no crate of it uses. Naming the package asked for added nothing
but another value for Cargo to build again on.

## Decision

**The programs are the workspace's: `RUST_JS_APPS` lists, by directory,
each of its members no crate of it uses, whichever packages a build asks
for. `rust-js build` builds every member of a workspace's root, at once,
or the packages `--package` names, as often as Cargo's `-p`, all in one
`cargo check`.**

- `cargoBuild` answers `apps` from Cargo's metadata of the workspace, the
  same for every build of it; the wrapper compiles a crate as a program
  where `RUST_JS_APPS` lists its `CARGO_MANIFEST_DIR`, as `RUST_JS_CRATES`
  lists the crates it compiles. It's recorded in the dep-info as
  `RUST_JS_APP` was: a crate one build checked is done for the next,
  whichever package that names, and the list changes, and every crate is
  built again, only where the workspace's graph does.
- `checkCargo`'s `packageNames` are Cargo's `-p`s; none is Cargo's own
  default, the manifest's package or each member of a workspace's root.
  `js` is the one package's built, where one is. Each package named must
  be compiled; a workspace's root needs none of its own.
- `rust-js build`, `clippy` and `test` take `--package` as often as
  Cargo's `-p`: `test` runs each package's harness, built one after
  another, which share what they check.
- React's port regenerates in one `rust-js build`,
  `scripts/react-in-rust.ts regen`'s, of the workspace or the crates named;
  the loop of one build per crate is gone.

## Why

- **One answer for which crates are programs**, of the workspace's graph:
  a crate's JS is the same however it's built, so building it twice is
  never needed and its JS in source doesn't depend on what was built last.
- **Cargo schedules the crates**: the dependencies once, and the crates
  that don't use each other, react-dom's and the five Flight servers', on
  the machine's cores at once, as `cargo check` of a workspace always has.
- **Cargo's own selection**, `-p` as often as needed and the workspace's
  members by default: nothing to learn, and nothing a rust-js flag adds.

## Consequences

- An editor's check at a workspace's root, `rust-js-check` (ADR 0222),
  compiles each member nothing uses as a program, as its build does, where
  it had compiled every crate as a library.
- The workspace test builds two packages, then the whole workspace, a
  crate checked once throughout, and `rust-js build` at the root and with
  two `--package`s writes every `.js`, `.d.ts` and map byte for byte as
  each package's own build does.
- React's port, regenerated the old way and this way from the same
  commits, rust-js 8a5b3afd9 and react-in-rust 4a005d8: all 2,630 `.js`,
  `.jsx`, `.d.ts` and map files the same, byte for byte, and the same 62
  differing from what the pin committed. On the M3 Max, beside another
  job's regeneration (2026-11-02): 475 s cold, 931 s of CPU, up to seven
  crates checked at once, and 1.7 s warm, every crate done; one build per
  crate took 2,186 s cold, 2,222 s of CPU on one core, and 2,405 s warm.
- A build that fails writes nothing, as one package's always has: where
  one crate of the port doesn't compile, the others' JS isn't diffed in
  that run, and the failure is Cargo's, naming the crate.
- A build of several packages unifies their features, as Cargo does: a
  crate two of them use with different features is built once, with both,
  where one at a time built it twice.
