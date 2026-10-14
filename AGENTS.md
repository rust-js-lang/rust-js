# rust-js: north star

**Rust in. Readable JavaScript out.**

A compiler that turns Rust into JavaScript people would have written by hand.
The longer-term goal is Rust across the stack: native Rust on the server,
rust-js on the frontend, shared types between them, with Scala.js-level
correctness, library support and interop. The first production app is a
milestone on the way.

**Be ambitious.** We are setting out to change how people build for the web,
so don't shy away from a big change. A representation reworked across the
compiler, a new analysis, a crate rebound from scratch: if it serves the
north star and keeps every promise below, do it, whatever it takes. The
size of a change is never a reason to settle for less; only the vision and
the promises are. Make it as safe as it is bold: a failing test first, the
corpus and mutations as the net, an ADR that says why.

## Exactly one way to do each job

**This rule comes before every other.** For any job there is exactly one
way to do it. Two ways to do the same job is a red flag: stop, and resolve
it before going on with anything else.

rust-js is a tool AI coding agents write against. An agent needs one answer
it can't get wrong, not a choice; every second way is a chance to pick the
wrong one, and a second thing to keep correct. The rule holds everywhere:

- **What a program writes:** one attribute, one macro, one API, one way to
  declare a binding, one option, one flag, one config key for each thing.
- **What rust-js writes:** one form of output for each construct.
- **Inside rust-js:** one module, pass, helper, constant or code path for
  each concept; one way a host builds; one list of each set of things; and
  one place that answers each question (the next rule).

When you meet two ways, whatever you were doing:

1. **Stop.** Don't build on either, and don't add a third.
2. **Choose one** by the north star: the one that covers every case,
   reads best, and keeps Rust's checks.
3. **Remove the other**: migrate what uses it, and make what's left of it
   a compile error, or a test failure, that names the one way.
4. **Hold it**: a test, often [the architecture test](test/architecture.test.ts),
   so the second way can't come back, and an ADR that says which one and why.
5. **Then go on** with the task you stopped.

Never add a synonym, an alias, an option, a fallback or an "also accepts"
for taste or convenience: people need preferences; an agent needs one
answer. Before adding a form, find the one that does the job and extend
it. If removing the second way seems out of proportion, stop and ask the
user; never leave both standing.

## One place answers each question

Every question the code asks has exactly one place that answers it: what a
value is in JS, whether a function is a binding, whether a check is written,
where a module's file goes, which old files a build may remove, which crates
rust-js ships. Every code path that needs the answer asks that place. None
works it out again, copies its condition, or keeps its own list.

Two answers to one question drift apart, and then two code paths behave
differently without anyone knowing: each is right alone, and the bug lives
in the gap. rust-js has had them: whether an `Assert` is written was decided
both by the count of reads and by the terminator, so a check was counted
but never made; where a module's JS goes in source was the compiler's rule
and the hosts' copy of it; a Vite build looked for committed JS where Next's
didn't.

- **Name the question, give it one owner**: a function whose name is the
  question, in the module whose concept it is, and route every path through
  it. The [architecture test](test/architecture.test.ts) holds who may ask.
- **Across a boundary**, the compiler and a JS host, Rust and TypeScript:
  the owner reports the answer (a manifest field, a flag, a catalog), and
  the other side reads it. Where it truly can't, one side's copy is held
  equal to the other by a test that runs both.
- **Spot the red flag**: the same condition in two `if`s, a list written
  twice, a comment that says "as X does" or "mirrors", a value derived in
  one place and derived again in another. Treat it as two ways to do one
  job: stop, give the question its one place, route every path through it,
  hold it with a test, then go on.

## Principles

- **Each piece understood on its own.** A module owns one responsibility
  and the concepts that job needs, no more, behind a boundary nothing
  leaks across, so you can read, change and test it without holding
  every other concept in your head. That's what keeps a change small and
  safe, and the codebase open to people and AI coding agents alike. Where
  a pass works around what a later phase does, or a module spells another's
  representation, give the concept one owner, as each question has one
  place (above), and have [the architecture test](test/architecture.test.ts)
  hold it.
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
what rust-js lacks. The port is not the goal; what it uncovers is. A port
that lands with a gap worked around has failed at its only job, however
well it runs.

- **Anything short stops the port.** Stop porting and fix rust-js, with a
  failing test, the fix, its mutations and an ADR, when a port meets any of
  these:
  - a feature rust-js lacks, or an error where Rust is fine;
  - a binding missing from the js, webapi or react crates, or one that
    types a value more narrowly or loosely than TypeScript does (an
    `any`, a missing `| undefined`);
  - generated JavaScript that doesn't read as the original does: a
    rename, a box, a helper, a cast, a copy, a lost name, a statement
    where the original has an expression;
  - Rust that has to be bent to get there: a narrower type than the
    original's, a leak, a restructured function.

  Then write the port the natural way, on the fixed rust-js.
- **Never work around rust-js in a port**: no local stand-in bindings for
  what a crate should bind, no `#[link_name = "this"]` casts, no
  restructured or narrowed Rust to dodge an error, no accepting noisy
  output. A workaround hides the gap the port exists to find.
- **Don't decide alone that a gap is acceptable.** "Cosmetic", "correct
  anyway" and "worth revisiting later" are not reasons to keep porting. If
  fixing it seems out of proportion, stop and ask the user; record what
  they decide.
- **Read every generated file against its original before committing a
  port**, line by line. Each difference is fixed in rust-js first, or the
  user has accepted it by name.
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
