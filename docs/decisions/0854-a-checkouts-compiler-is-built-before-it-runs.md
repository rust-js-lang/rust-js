# 0854. A checkout's own compiler is built from its sources before it runs

Status: Accepted. Amends [0084](0084-owned-phases-and-host-boundaries.md)'s
compiler discovery, its checkout fallback, which
[0412](0412-rust-js-build-is-its-own-process.md)'s `rust-js build` and
[0222](0222-editor-check.md)'s editor check run.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): which compiler a host
runs, not what a program means. But a stale one gives a wrong answer, the
worst bug: JS the sources no longer write, and refusals they no longer make.

## Context

`findCompiler(root)`, the one answer to which compiler a host runs, is
`RUST_JS_COMPILER` where it names one, else the app's `@rust-js/native`,
else the rust-js checkout's own build, `target/debug/rust-js`. An app built
from a checkout, as the React port is (`node ../rust-js/tooling/cli.js
build`), took that path, and ran whatever binary was there: the one the
last `cargo build` left. After `git pull` and `cargo build --release`, the
debug binary was older than the sources, and `rust-js build` wrote stale JS
and stale refusals, with nothing to say so. A bug already fixed was chased
twice, for hours.

## Decision

**The checkout's own compiler is built from its sources as they are before
it's run.** `findCompiler` runs the checkout's `build:compiler` script, in
the checkout, and only then answers with `target/debug/rust-js`:

- **`build:compiler` is the one definition** of the compiler's build,
  `cargo build`. The checkout's `bun run build` runs it, then React's
  metadata for the tests (`react/build.sh`); the host reads it from the
  checkout's `package.json` and runs it with `sh`, as a package manager
  does, so the two can't drift apart.
- **Cargo decides whether it's current.** Its fingerprint is of every
  source, `Cargo.lock`, `build.rs`, the toolchain and the flags: nothing
  changed, it does nothing, in 0.15 s; a source changed, it rebuilds.
- **Cargo's lock serializes hosts.** Two hosts at once, `rust-js build`
  and the editor's check, say: one builds, the other waits ("Blocking
  waiting for file lock on build directory"), finds nothing to do, and
  both run the same, current binary.
- **As a shell in the checkout builds it**: `CARGO_TARGET_DIR` is the
  checkout's `target`, where the compiler is run from, whatever the app's
  build uses; `RUSTC_BOOTSTRAP` is `.cargo/config.toml`'s, as another, a
  test's say, rebuilds every dependency (as `test/support.ts`'s
  `buildCompiler` does).
- **What the build says is on stderr**, as Cargo prints it, its progress
  too. stdout is the editor check's Cargo messages, for rust-analyzer.
- **A build that fails stops the host**, after what the build said:

  ```
  rust-js: building /path/to/rust-js/target/debug/rust-js from this checkout's sources failed, as it says above; the one an earlier build left isn't run: fix it, or run `bun run build:compiler` in /path/to/rust-js
  ```

  The binary an earlier build left is of other sources, so it's never run
  instead.

**`RUST_JS_COMPILER` is run as it is**, even when it names the checkout's
`target/debug/rust-js`. Whoever names a compiler builds it: the tests'
`buildCompiler` once per run, `bun run test`'s `cargo build --quiet`, a
mutant's build ([ADR 0093](0093-mutations.md)), a release's
`target/release/rust-js`. Building what a variable names because of where
its path is would be a second rule read from a path's shape; and every test
file building it, side by side, would relink the binary under the others.
**`@rust-js/native` is run as it is**: a release, not a checkout's. An
installed `@rust-js/build` is of no checkout (its directory has no
`@rust-js/workspace`), so nothing is built and its path is of no compiler,
as before.

`createNativeBuilder` takes the compiler; it no longer finds one itself, so
each host that runs the checkout's asks `findCompiler`, which builds it.

## Why

Correctness first: a host that runs a binary of other sources gives answers
the sources don't, and says nothing. Least surprise: a checkout's command
runs that checkout's code, as `cargo run` does. The build's cost is Cargo's
no-op, 0.15 s, against an app's build of seconds; after a pull it's the
build the developer would have had to run anyway, its progress shown.

The hosts that run `findCompiler` are `rust-js build` and the editor check.
Neither runs in a dev server's loop: an app's dev server never compiles its
Rust ([ADR 0412](0412-rust-js-build-is-its-own-process.md)). The
playground runs its WASI compiler, built by `wasm/build.sh`, and never asks
`findCompiler`. So no host needs the alternative below.

## Alternatives

- **Refuse when the binary is older than its sources**, naming the command.
  It makes the developer run what the host can, each time. And the check
  would be a second authority over freshness, an approximate one: an mtime
  comparison misses a toolchain change, a `Cargo.lock` update, a `build.rs`
  input, files restored with their old times (`tar`, `rsync -a`, `cp -p`),
  and fires on a `touch` that changed nothing. Cargo's fingerprint is the one that's right.
- **Run the whole `bun run build`.** It rewrites React's metadata for the
  tests, `target/libreact.rmeta`, `libwebapi.rmeta` and `libjs.rmeta`, each
  time, 1.7 s, though an app's build reads none of it (an app's Cargo
  builds the react crate itself), and it needs Bun, which an app built with
  Node doesn't.
- **`cargo build` written in `tooling/build.js`.** A second definition of
  the compiler's build, to drift from `bun run build`'s.
- **Build an explicit `RUST_JS_COMPILER` under the checkout's `target/`.**
  Rejected above.

## Consequences

- `findCompiler` is async; `rust-js build` and `rust-js-check` await it.
- A checkout's `rust-js build` and editor check print Cargo's `Finished`
  line first, and its progress after a pull.
- A build of the checkout while an app's build runs can still relink the
  compiler between two of that build's crates, as it could before; this
  decision doesn't change it.
- `test/host-boundaries.test.ts`, "a checkout's own compiler is built from
  its sources before it runs, and no other is", holds each rule above,
  with a stand-in `build:compiler` of a checkout made for it: built first,
  where and as a shell builds it, its output on stderr; a failed build's
  message, its old binary not run; `RUST_JS_COMPILER`, `@rust-js/native`
  and an installed `@rust-js/build` built nothing.
