# rust-js: north star

**Rust in. Readable JavaScript out.**

A compiler that turns Rust into JavaScript people would have written by hand.
The longer-term goal is Rust across the stack: native Rust on the server,
rust-js on the frontend, shared types between them, with Scala.js-level
correctness, library support and interop. The first production app is a
milestone on the way.

## Principles

- **Keep Rust's checks.** rustc owns types, traits, ownership, borrowing and
  diagnostics. Never bypass them to make a feature compile.
- **Generated code is a product.** Names, control flow, modules, JSX,
  formatting and source maps must read well to JavaScript developers.
  Inspect the output when changing the compiler.
- **Use JavaScript's building blocks**: objects, arrays, functions, promises,
  ES modules. Emit helpers only where behavior needs them.
- **Make semantics explicit.** Keep the contract: evaluation order, side
  effects, copying, overflow, errors. Differences from native Rust are
  deliberate, documented and tested; readability never excuses an accidental
  one.
- **Decide a disagreement by what the program can observe**
  ([ADR 0262](docs/decisions/0262-when-rust-and-js-disagree.md)):
  - it can't: JavaScript's, and say why;
  - only where it's rare and harmless: JavaScript's, listed under the ADR's
    Consequences;
  - results would change: Rust's, paid for in output (`>>> 0`, `$byteLen`);
  - Rust's can't be kept: a compile error, never silent drift.

  The program's meaning is rustc's: never make wrong Rust work in JavaScript
  for nicer output. Every decision states which case it's in.
- **Reject unsupported features clearly**: a useful compiler error, never an
  approximation or partial output.
- **Interop is fundamental.** Browser APIs, React, npm, Vite and JavaScript
  callers are core. Preserve public representations and interfaces, and keep
  the native and browser compilers consistent.

## Ports drive rust-js

We port TypeScript and JavaScript projects to Rust, react.dev first, to find
what rust-js lacks. The port is not the goal; what it uncovers is.

- **A limit stops the port.** When a port needs a feature rust-js lacks, a
  binding, or gets JavaScript that doesn't read as the original does, stop
  porting and fix rust-js: a failing test, the fix, its mutations, an ADR.
  Then write the port the natural way.
- **Never work around rust-js in a port**: no local stand-in bindings, no
  restructured Rust to dodge an error, no accepting noisy output. A
  workaround hides the gap the port exists to find.
- **A port is faithful**: the same behavior, and output as close to the
  original as its Rust allows.

## Making changes

Follow [CONTRIBUTING.md](CONTRIBUTING.md): how a change is made, proved and
recorded. [The architecture](docs/architecture.md) says where it goes, and
[its test](test/architecture.test.ts) holds the boundaries.

Commit on `main` and push directly: no branches, no pull requests. If a push
fails, keep committing locally and push later
([DEVELOPMENT.md](DEVELOPMENT.md#working-on-main)).

## Five minutes per local command

A hard rule: every command on this Mac runs with a timeout of at most five
minutes, the tool's own or `timeout 300`. It wins over anything else here or
in CONTRIBUTING.md.

- **Longer work goes to CI**: push `main`, then `bun run ci:check`. The whole
  suite, all mutations, rustc's whole suite and the WASM build are CI's.
- **A timed-out command isn't rerun with more time.** Split it (one file,
  `-t <name>`, one mutation, one folder of rustc's tests) or send it to CI.

## Where checks run

Every check runs on the Mac (its malware scan is off for the apps that run
them, [DEVELOPMENT.md](DEVELOPMENT.md#the-mac-with-the-scan-off)) within five
minutes; longer runs are CI's:

| To | On the Mac | On CI |
|---|---|---|
| Test | `bun test test/<file>.test.ts -t <name>` | `bun run test` |
| Bless snapshots | `BLESS=1 bun test test/<file>.test.ts -t <name>` | `bun run ci:bless` applies its patch |
| Mutations ([ADR 0093](docs/decisions/0093-mutations.md)) | `bun scripts/mutations.ts <name>` | the changed ones; all before a release |
| rustc's tests ([ADR 0089](docs/decisions/0089-rustc-tests.md)) | `bun run test:rustc drop/ closures/` | all, lists blessed by `bun run ci:bless` |
| Generated programs ([ADR 0092](docs/decisions/0092-generated-programs.md)) | | `gh workflow run "rustc tests" -f fuzz_start=1000 -f fuzz_seeds=600` |
| The playground's compiler | | the check's `wasm` job |

Before pushing: `bun run typecheck`, `bun run fmt:check`,
`cargo clippy --locked -- -D warnings`, and the change's tests and mutations.
Then `bun run ci:check` runs [the check workflow](.github/workflows/check.yml).
Workflows are started by hand ([DEVELOPMENT.md](DEVELOPMENT.md#workflows)).
