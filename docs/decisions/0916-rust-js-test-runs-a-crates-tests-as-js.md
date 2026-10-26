# 0916. `rust-js test` runs a crate's `#[test]`s as JS, through Cargo

Status: Accepted. Extends [0026](0026-testing.md) and
[0412](0412-rust-js-build-is-its-own-process.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): N: a command; what the
tests do is what the JS rust-js writes for them does.

## Context

ADR 0026 made a crate's own `#[test]`s its tests: `rust-js --test lib.rs
-o out/lib.js` compiles them, its harness left out, and writes
`lib.test.js`, which calls the runner's `test()` for each. But the
compiler run alone doesn't find the crates Cargo builds, and `rust-js
build` compiles no tests, so a crate with a dependency had no way to run
its tests as JS. The guide said so ("`rust-js build` doesn't compile tests
yet"). TheAlgorithms/rust's tests, ported to check rust-js against native
Rust, use `rand`, `num-bigint`, `nalgebra` and `quickcheck`: 36 of its
modules couldn't be checked.

## Decision

**`rust-js test [--package <name>] [-- <bun test flags>]`**, beside
`rust-js build` and `rust-js clippy` (ADRs 0412, 0888), as `cargo test` is
beside `cargo build`. Decided with the user (2026-10-26), over a `rust-js
build --tests` flag: build and test are each one job.

- **Cargo builds the tests**: `cargo check --profile test --lib` of the
  package, with rust-js as its wrapper, as `rust-js build` runs it, so each
  dependency is compiled to JS as a build compiles it, of crates.io's
  where rust-js can (ADR 0415).
- **The package's test build is rust-js's `--test`**: Cargo gives rustc
  `--test` and no crate type for it (`builds_tests`), which the wrapper
  turns into rust-js's own `--test`, rustc's not passed twice, writing its
  JS and `lib.test.js` in its directory of the target, `rust-js/<crate><extra>/`.
  A dependency's test build isn't asked for; were it, it's rustc's.
- **Out of source**: a target directory of its own,
  `node_modules/.cache/rust-js/test`, or `CARGO_TARGET_DIR`. Nothing is
  written beside the Rust, as `rust-js clippy` writes nothing.
- **bun runs it**: `bun test <lib.test.js>`, what follows `--` given it
  too, `-t <name>` say; its exit code is the command's.

## Consequences

- Only a library's unit tests: an integration test, `tests/*.rs`, a
  crate of its own, isn't built yet.
- Held by `test/rust-js-test.test.ts` (a path dependency's JS, a passing
  and a `should_panic` test, a failing one's exit code and message, `-t`)
  and the mutations `cargo-tests-rustcs` and `cargo-tests-test-twice`.

## Amendment: a unit's directory is Cargo's to give

Cargo gives a package the same unit directory wherever its path is, so two
crates of one name sharing a `CARGO_TARGET_DIR`, or a project moved, write
JS to the same `rust-js/<crate><extra>/`, and the second build failed: its
manifest "belongs to another compilation". **A crate Cargo builds replaces
what another wrote there**; the guard, which keeps one compilation from
overwriting another's files, holds outside Cargo, where the directory is
the user's. Held by the test "two crates of one name at two paths, one
target, each run their own tests" and the mutation
`cargo-unit-of-another-refused`.

