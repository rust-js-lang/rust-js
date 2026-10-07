# rust-js: north star

**Rust in. Readable JavaScript out.**

Build a compiler that lets people write Rust and get JavaScript they would
have been willing to write by hand. The longer-term goal is Rust across the
application stack: native Rust on the server, rust-js on the frontend, and
shared types and data models between them.

Aim for Scala.js-level correctness and completeness for full-stack Rust:
broad language and library support, reusable shared crates, and dependable
interop and tooling. The first production app is an intermediate milestone.

## Principles

- **Keep Rust's checks.** rustc owns types, traits, ownership, borrow checking,
  and diagnostics. Never bypass its checks to make a feature compile.
- **Treat generated code as a product.** Names, control flow, modules, JSX,
  formatting, and source maps must be understandable to JavaScript developers.
  Inspect the output when changing the compiler.
- **Use JavaScript's building blocks.** Prefer objects, arrays, functions,
  promises, and ES modules. Emit helpers only where the supported behavior
  needs them. Keep compiler code and generated code simple.
- **Make semantics explicit.** Preserve the established contract, including
  evaluation order, side effects, copying, overflow, and errors. Differences
  from native Rust must be deliberate, documented, and tested. Readability
  does not justify accidental behavior changes.
- **Reject unsupported features clearly.** Report useful compiler errors;
  never silently approximate behavior or publish partial output on failure.
- **Make interop fundamental.** Browser APIs, React, npm, Vite, and JavaScript
  callers are core concerns. Preserve public representations and interfaces.
  Keep the native and browser compilers consistent.

## Making changes

Read and follow [CONTRIBUTING.md](CONTRIBUTING.md) for every contribution:
correctness first, the architecture principle, how a change is made,
proved and recorded, and what to run before pushing. The
[architecture](docs/architecture.md) says where a change goes, and which
boundaries [its test](test/architecture.test.ts) holds it to.

A PR is often merged right after it's opened, and sometimes not: expect either, and check its state before building on its branch. A merged PR's branch is deleted, so don't push to it again; sync `main` instead.

## Five minutes per local command

A hard rule: every command run on this Mac is given a timeout of at most
five minutes, the tool's own or `timeout 300` in front of it. Nothing runs
here without one.

- **What may take longer goes to CI**, not here: push the branch and start
  `bun run ci:check`, or the [workflow](.github/workflows) that runs it.
  The whole suite, all mutations, rustc's whole suite and the WASM build
  are CI's ([DEVELOPMENT.md](DEVELOPMENT.md)).
- **A command that times out isn't run again with more time.** Split it, a
  file's tests, `-t <name>`, one mutation, a folder of rustc's tests, or
  send it to CI.
- **This wins** where the rest of this file, or CONTRIBUTING.md, asks for a
  run here that takes longer.

## Develop on the Mac

macOS checks each newly built binary before its first run, one at a time,
which made the tests that build native programs, the corpus, rustc's tests,
generated programs and mutations, many times slower: rustc's `drop` tests
took 23 seconds against 1 on Linux. With the app that runs them added to
Developer Tools, there's no check
([DEVELOPMENT.md](DEVELOPMENT.md#the-mac-with-the-scan-off)), so every check
runs here, on the Mac, within the five minutes above, and what takes longer
runs on CI:

| To | On the Mac | On CI |
|---|---|---|
| Test | a module's, `bun test test/<file>.test.ts -t <name>` | the whole suite, `bun run test` |
| Bless snapshots | a module's, `BLESS=1 bun test test/<file>.test.ts -t <name>` | its patch, applied with `bun run ci:bless` |
| Put a known bug back, and see its tests catch it ([ADR 0093](docs/decisions/0093-mutations.md)) | `bun scripts/mutations.ts <name>` | the changed ones, and all of them before a release |
| Check rustc's tests against the known failures ([ADR 0089](docs/decisions/0089-rustc-tests.md)) | some: `bun run test:rustc drop/ closures/` | all of them, and their lists' patch, applied with `bun run ci:bless` |
| Run generated programs ([ADR 0092](docs/decisions/0092-generated-programs.md)) | | `gh workflow run "rustc tests" -f fuzz_start=1000 -f fuzz_seeds=600` |
| Build the playground's compiler | | the check's `wasm` job |

Before pushing a change ([CONTRIBUTING.md](CONTRIBUTING.md)), run
`bun run typecheck`, `bun run fmt:check`, `cargo clippy --locked -- -D
warnings`, the change's tests and its mutations here; then `bun run
ci:check` runs everything [the check workflow](.github/workflows/check.yml)
does ([DEVELOPMENT.md](DEVELOPMENT.md)). rustc's known failures are Linux's,
as CI finds them, so their lists are blessed there and committed with the
change that moves them. A test the Mac or CI's x86 machines ignore is out of
scope.

The [workflows](.github/workflows) are started by hand: `bun run
ci:check`, and `gh workflow run "rustc tests"` with `-f bless=true`,
`-f mutations=true`, or `-f fuzz_start=1000 -f fuzz_seeds=600`.

To set the Mac up once: rustup, with the toolchain and components
[rust-toolchain.toml](rust-toolchain.toml) pins, and the `wasm32-wasip1`
and `wasm32-unknown-unknown` targets; the Bun version the workflows set
up; Node 24; then `bun install` and `bunx --bun playwright install
chromium`. Add the app that runs the checks, and a terminal you run them
from, under **System Settings → Privacy & Security → Developer Tools**.
