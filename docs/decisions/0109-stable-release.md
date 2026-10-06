# 0109. Pin a stable release: rust-js checks code as the Rust most people run does

Status: Accepted. Amends [0003](0003-pin-nightly-toolchain.md): the pin is a
stable release, `1.99.0`, where it was `nightly-2026-03-25`. Amended by
[0110](0110-stable-syntax.md): rust-js's syntax is stable Rust's, so its
session allows no unstable feature, and rustc refuses a crate's own itself.

## Context

rust-js checks a program with the rustc it links, so its pin is the Rust a
user's code can be. Rust is backward compatible, so a newer rustc takes all
code an older one did: what matters is not being older than the Rust users
run. Most of them run the latest stable, which rustup keeps them on, and
crates declare a minimum Rust version at or below it. The pin was a nightly
of 1.96 when stable was 1.98: code that used what 1.97 and 1.98 stabilized
was an error in rust-js.

A nightly was pinned because rustc's internal crates, `rustc_private`, are a
nightly's. A stable release ships them too, in its `rustc-dev` component, and
`RUSTC_BOOTSTRAP` lets a crate use them.

## Decision

**The pin is a stable release, `1.99.0`, and moves with each one.**

- **rust-js's own crate uses rustc's internals**, by `RUSTC_BOOTSTRAP=rust_js`
  in `.cargo/config.toml`: that crate's name alone, so no other crate is
  built with unstable features.
- **Its syntax is rustc's unstable features** (ADR 0039): `register_tool` and
  the rest, which a stable release allows no crate. rust-js's session allows
  them, and refuses a crate's own `#![feature]`, as that release does, with
  its error, E0554: a program is stable 1.99.0's Rust. `RUSTC_BOOTSTRAP=1`
  allows it, as for rustc. A crate may name rust-js's own features, which
  it turns on anyway.
- **The binding crates are built so too**, each by its build script:
  `RUSTC_BOOTSTRAP=js`, `webapi`, `react`. Since ADR 0112, rust-js compiles
  them, and they need none.
- **The tests' programs, and rustc's**, which use rustc's features natively
  too, are built with `RUSTC_BOOTSTRAP=1`: `.env.test`, which `bun test`
  loads for every process it starts, and the rustc suite, as rustc's own CI
  runs its tests.
- **The playground's rustc is the release's source**, its version stamps the
  release's, `1.99.0 (b940084d7 2026-09-28)` on the `stable` channel, or it
  refuses the shipped `core`'s metadata. Its lockfile is rustc's own, with
  rust-js's crates' where they need a newer version.

## Why

- **The release is the Rust users run**, so rust-js takes what their
  `cargo build` takes, and refuses what it refuses, with its words.
- **A release is tested**, where a nightly is a day's snapshot.
- **The escape hatch is narrow**: the one crate, rust-js's, uses internals,
  and a user's code gets no feature it didn't have on stable.

## Consequences

- **The upgrade from 1.96**: instantiating a generic value now returns it
  marked as maybe needing normalization, `Unnormalized`, which rust-js takes
  as it was, `.skip_normalization()`; an alias type's kind carries its
  `DefId`; `EarlyBinder::bind` and `Ty::new_projection` take more.
- **A `TryFromIntError` is its message**, as each parse error is (ADR 0063):
  1.98's shows its kind, `TryFromIntError(PosOverflow)` where 1.96's was
  `TryFromIntError(())`, and since 1.99 its message tells the kind too,
  "number too large to fit in target type" or "too small", where it was one
  sentence for both. `{:?}` takes the kind from the message, as a
  `ParseIntError`'s does. `==` of a parse error, which its derived
  `PartialEq` compares by kind, is in.
- **The playground's patch for dynamic libraries** moved with rustc's code,
  to `rustc_metadata/src/host_dylib.rs`.
- **Found by rustc's tests at 1.98.1**, as the upgrade's own:
  - `size_of_val` of a generic `async fn`'s future is laid out as codegen
    lays it out, in a fully monomorphic environment: in the function's,
    1.98 finds it too generic.
  - An externally implementable item, 1.98's `#[eii] static HELLO: u64;`,
    which the linker makes another item, is refused: its JS read a name
    nothing defines.
  - 1.98's `assert_eq!` has a block of its own, so a temporary of its
    operands ends with that block, not the statement: one with a destructor,
    `assert_eq!(BAR.0, 456)` of a `const BAR: A` with a `Drop`, is refused,
    where rust-js drops only at a statement's end or a `let`'s.
- **rustc's tests are the release's**: rustc renamed and took out some,
  so a bless of another rustc than the inventory's takes the new ones, and
  a known failure no longer a test goes with its diff.
- **The playground's rustc and oxc would share a `hashbrown`** now, 0.17:
  rustc's `nightly` feature, for `may_dangle` in drop checks, which rustc's
  arenas need, would make oxc's std's unstable `Allocator`, which oxc's arena
  isn't. So the playground's rustc takes the same release from hashbrown's
  repository, `v0.17.0`, by a patch: to cargo, a package of its own.
- **The upgrade to 1.99**: rustc's lang items moved to `rustc_attr_ir`,
  which `rustc_hir::attrs` re-exports; `LocalModDefId` is `LocalModId`;
  `predicates_of` is `clauses_of`, its list `clauses`. The tools rustc
  knows are two lists now, an attribute's and a lint's: `rust_js` is an
  attribute's. A function item's generic args are bound now: a body's
  never have bound vars, so rust-js reads them unbound, `fn_def`, as
  rustc's own MIR building does.
- **1.99's messages**: a radix past 36 panics naming
  `from_ascii_bytes_radix`, where it was `from_ascii_radix`.
- **A JS method named as libc's functions are**, `open`, `close`,
  `write`, is one in native code of the runtime's symbols, which 1.99's
  `invalid_runtime_symbol_definitions` checks, an error: the webapi crate
  allows it, as it allows `clashing_extern_declarations`, for a native
  `cargo check` of an app.
- **The playground needs one patch fewer**: 1.99's rustc grows no stack,
  `ensure_sufficient_stack` is gone, so `stacker`, and `psm` with its
  precompiled object, which Apple's `ar` couldn't archive, are too.
- **Found by rustc's tests at 1.99.0**: `IntoIterator` of a
  `Box<[T; N]>`, 1.99's, which `Box::new([1, 2]).into_iter()` now calls
  where it took a `&` of the array. A box is its array, so by value it is
  the array's; through a `&mut`, a handle on the box (ADR 0099), the
  handle's place, and `for n in &mut b` of numbers an index loop, as of an
  array.
- **Still unstable**: rust-js's syntax, which a plain rustc refuses without
  `RUSTC_BOOTSTRAP`. Moving it to stable Rust is to come.
