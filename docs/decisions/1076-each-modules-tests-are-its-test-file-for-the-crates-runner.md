# 1076. Each module's tests are its `.test.js`, written for the runner the crate names, Node's by default

Status: Accepted. The user's decision (2026-11-03). Amends
[0026](0026-testing.md): a test file imports its runner's `test`, not a global,
and is one beside each module's JS that has tests, not one `<output>.test.js`.
Amends [0916](0916-rust-js-test-runs-a-crates-tests-as-js.md): `rust-js test`
runs the runner the crate names, Node's by default, not bun, and builds the
crate's integration tests too. Amends [0027](0027-real-browser-tests.md): its
runners are given a build's manifests. Amended (2026-11-04, #597, the
user's decision): a test build compiles the package's library as `rust-js
build` does, not as a library, and its integration tests are held to what it
decided; see "Amendment: a program's tests run the JS it ships" below.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, a command and a file
layout: what each test does is what the JS rust-js writes for it does, under
any runner. Where runners differ, it's their reporting: a promise rejected
and never handled fails Node's and Vitest's run, not the test, as bun's and
Playwright's fail the test (ADR 0027); Rust has no such failure.

## Context

`rust-js test` ran `bun test` on one generated `lib.test.js`, which called the
runner's global `test()` once for each `#[test]` of the crate, each module's
tests imported from where its path put it: `shapes::tests` at
`shapes/tests.js`. Two things were wrong with that for a user:

- **Bun is not what a user has.** Node is: npm and Vite need it. And
  `node:test` has no global `test`, so the file ran under bun and Vitest
  with `globals: true` alone.
- **One entry isn't how JS projects lay tests out.** A test file sits
  beside the module it tests, `shapes.test.js` beside `shapes.js`, and every
  runner finds it by that name, `node --test` too.

## Decision

The user decided (2026-11-03), and the coordinator took the recommended
option on each question it asked (Q1 to Q4 below).

**The runner is one setting**, `Cargo.toml`'s, where a crate's other rust-js
settings are (ADR 0117):

```toml
[package.metadata.rust-js.test]
runner = "vitest" # "node", the default, "bun", "vitest" or "jest"
```

Per package. An unknown key or runner is refused, the error naming the
four. A file compiled alone, `rust-js --test x.rs`, reads the nearest
`Cargo.toml` above it, as its other settings are; the playground, which has
none, is Node's. There's no flag besides it, nor any other name for it.

- **The compiler owns what each runner's `test` is imported from**
  (`src/settings.rs`'s `Runner::module`): `node:test`, `bun:test`, `vitest`,
  `@jest/globals`. Each test file imports it, by a name nothing of its
  module's is, `test as test$1` beside a `fn test` of the tests'.
- **The manifest records it**, `"tests": { "runner", "module", "files" }`,
  and hosts read it there: `rust-js test` the runner and the files, the
  playground and the Playwright page the module, which they give their own
  `test` in place of.
- **`rust-js test` runs it** (`tooling/runners.js`, the command of each):
  `node --test`, `bun test`, `vitest run`, `jest`, what follows `--` before
  the test files. Each is given the files and where they are, as the target
  is in `node_modules` by default, which each leaves out of what it finds:
  Vitest `--dir`, Jest `--roots`, its haste map keeping what's there, and
  `--runTestsByPath`, with `NODE_OPTIONS=--experimental-vm-modules`, as
  Jest's ES modules need. Packages of one runner run together.

**Each module's tests are its `.test.js`**, beside its JS, by one rule:

- A module's test file is its JS's path with `.test.js`: `shapes.js`'s
  `shapes.test.js`, `lib.js`'s `lib.test.js`.
- A **module of tests** is one declared under `#[cfg(test)]` (a `cfg` that
  needs `test`, `test` or an `all` of it, as rustc keeps it, `CfgTrace`, not
  its name) that has `#[test]`s of its own, within no other. A module's one
  module of tests is its parent's test file, no file of its own:
  `shapes::tests`, inline or `shapes/tests.rs`, is `shapes.test.js`, its
  helpers and its tests there, each registered by its name.
- Every `#[test]` is registered, under its Rust path, `test("shapes::tests::
  squares", squares)`, in its module's test file, or, inside a module of
  tests, in that module's parent's: `shapes::tests::nested::of_one` in
  `shapes.test.js`, `nested` its own file, `shapes/tests/nested.js`.
- A `#[test]` written in any other module stays in its module's JS, which
  its siblings may call it from, and its module's test file imports and
  registers it: one of registrations alone where it has no module of tests.
- **The exception, Q2(e):** a module with several modules of tests,
  `mod tests` and `mod proptests`, can't have both as its one file: each is
  its own file, `tests.js` and `proptests.js`, and its test file registers
  theirs.
- A `#[cfg(test)]` module of no `#[test]`s, the tests' fixtures, is a module
  as any other, its own file, which no runner runs.
- **Integration tests**, `tests/quadruples.rs`, are crates of their own:
  `rust-js test` builds them, `cargo check --profile test --lib --test
  <each>`, its root `quadruples.js`, named as its file is, its tests
  `quadruples.test.js` by the same rule. The package's library is compiled
  as `rust-js build` compiles it, a program where no crate of its workspace
  uses it, and writes a contract they use (amended, #597, below).

No `lib.test.js` entry is written: a root's `lib.test.js` is its own tests'.
Private items the tests reach, `use super::*`, are exported, as before.
`cargo test`, native, is unchanged.

**`#[ignore]`** is `test.skip`, **`#[should_panic(expected)]`**
`() => $shouldPanic(f, "expected")`, a **`Result`** test `() =>
$testResult(f())`: each runner has `test` and `test.skip` of a name and a
function, and the outcome is the runtime's (ADR 0026).

**A DOM is the runner's own setup**, Q3: rust-js adds none. Node's is
`--import` of a module that registers happy-dom, `rust-js test -- --import
./test/dom.js`; bun's `preload` in `bunfig.toml`; Vitest's `environment:
"happy-dom"`; Jest's `testEnvironment`. Each finds its own config, as
`rust-js test` runs it in the app's directory. Real browsers (ADR 0027):
Playwright's page maps the manifest's module to its recording `test`;
Vitest's browser mode runs a crate's files whose runner is `"vitest"`, and
refuses another's, saying so.

## Alternatives

- **A `--runner` flag beside the setting, or a runner inferred from
  `package.json`'s devDependencies.** Two ways to say one thing, or a guess
  (AGENTS.md: one way).
- **Keep a global `test()`.** `node:test` has none, and the default would
  need a preload to make one.
- **Every runner's `test` through rust-js's own shim module.** A rust-js
  dialect where the runner's own import is what a JS project writes.
- **Two Rust modules in one JS file**: a module of tests and its nested
  modules, or a module's several modules of tests, flattened into its
  parent's test file. A JS module per Rust module is what lowering, linking
  and naming are built on; flattening needs names kept apart across modules.
  Nested modules keep their files; several modules of tests are the
  exception above.
- **Refuse a module's second module of tests**, Q2(e)(i). It's valid Rust.
- **Move a `#[test]` of another module into its test file.** One module's
  items in two files, its test file importing the rest, a module of its own
  the compiler would make up; its siblings may call it.
- **An integration test's root as its test file itself**, Q2(f). Only Cargo
  knows a target is one; `rust-js --test tests/foo.rs` alone can't, and the
  same Rust would be other JS by its host.
- **rust-js preloading happy-dom where the app has it installed**, Q3. A
  guess, and a second way to set up what the runner's own config does.

## Consequences

- Jest and `@jest/globals` are devDependencies, Q4, so each runner's run is
  tested: test/rust-js-test-runners.test.ts runs the same Rust tests under
  each, a skip, a `should_panic` of the right message, of the wrong one, and
  a `TypeError`, a `Result`'s `Ok` and `Err`, and an integration test.
  test/test-files.test.ts holds the layout, test/rust-js-test.test.ts the
  command's, and test/browser.test.ts the example apps' DOM tests under Node,
  the default. Mutations in `analysis/tests.ts`, `output.ts`, `settings.ts`
  and `cargo.ts` hold each decision.
- A manifest's `tests` is an object; one of an older build, a path, is
  refused, as `RUST_JS_TESTS` naming a test file is.
- Under Node and Vitest, a promise a test leaves rejected fails the run, not
  that test: Node names the test that left it.

## Amendment: a program's tests run the JS it ships (2026-11-04, #597)

The user's decision. Porting alien-signals' library tests found that a test
build, which compiled the package's library `--library`, refused what `rust-js
build` builds: a library guesses what any consumer gives it (ADRs 0163,
0521), so `computed<F>`'s getter boxed into a `dyn Fn` made every `dyn Fn`
drop, its `Rc<ReactiveNode>` counted, and eleven refusals followed. Where
library mode compiles, the tests run other JS than the build ships.

**A test build compiles a program as `rust-js build` does**, its JS byte for
byte, and **writes a contract too**, `--tested`, which the cargo wrapper
passes to an app, a crate `RUST_JS_APPS` lists, where `rust-js test` sets
`RUST_JS_TEST_BUILD` (`tooling/cargo.js`'s `cargoEnv`), recorded in its
dep-info as `RUST_JS_APPS` is. Its analysis is a program's (`pipeline.rs`:
`library` is `export_library && !program`); its contract says what it
decided as one. **Its integration tests, crates of their own that use it, are
held to each decision, refused at compile time where they'd break one**:

- **A value whose drop runs given to its type parameter**: its contract's
  `Drop` and `NoDrop` marks, as a library's; a consumer that gives more
  compiles its own instance (ADR 0914) or is refused (ADR 0190).
- **A closure holding what drops made one of its `dyn Fn`s**:
  `makes_dropping_fn_dyns`, refused (ADR 0626), as `tp::node(move ||
  loud.0.get())` is.
- **A count of an `Rc` it counts none of**: `counted_rcs`, refused (ADRs
  0844, 0852).
- **A `dyn` of its own trait of what drops**, where none of its implementors
  drops, so its code drops nothing of one (ADR 0320): its contract's
  `dyns_drop_nothing`, the coercion refused (`analysis.rs`, `closures::
  dropping_dyn_of`), the rule `drops::own_dyns_drop_nothing`, one for both.
- **A trait's constant read of a type parameter** that its dictionaries lack,
  carrying only what its own generic code reads (ADR 0106): its contract's
  `dictionary_consts`, the read refused, naming `<Ty as Trait>::C`, which
  rustc evaluates.
- **A borrow of a `RefCell` it borrows unchecked, held while its code runs**:
  `ref_cells.unchecked`, refused (ADR 0863).

A library's contract has no `dictionary_consts` and no `dyns_drop_nothing`:
its dictionaries carry every constant and its `dyn`s drop what they hold.
`rust-js build` is unchanged; ADR 0360's rule, a crate another of the
workspace uses is a library, holds for test builds too: an integration test is
no member of the workspace.

Case ([0262](0262-when-rust-and-js-disagree.md)): N where the test keeps the
program's decisions, its answer the shipped JS's; D, a compile error, where it
would break one, as native Rust's answer would then differ.

**Alternatives.** (A) Keep library mode: a library shaped as alien-signals is
refused, and its tests run other JS. (A') Decide every library by its own code
and hold its consumers to it: every library's JS changes, React's crates too;
not for this change. (C) Integration tests as `#[cfg(test)] mod tests` of the
library: not what a Rust crate writes, `tests/*.rs`.

**What holds it.** `test/rust-js-test-programs.test.ts`: the test build's
library JS is `rust-js build`'s, byte for byte, an `Rc` the value itself, and
each refusal above.
Mutations in `scripts/mutations/cargo.ts` (`cargo-tested-program-a-library`,
`cargo-tested-program-writes-no-contract`), `lower/pipeline.ts`
(`program-guesses-as-a-library`, `program-dictionary-consts-unsaid`,
`program-dyns-drop-nothing-unsaid`), `lower/analysis/programs.ts`
(`tested-program-const-read-unrefused`, `tested-program-dropping-dyn-unrefused`)
and `lower/library.ts` (`library-dictionary-const-lacked`,
`program-dyn-trait-keys-unread`). alien-signals' `tests/effect.rs`,
`effect_scope.rs` and `trigger.rs`, upstream's 21 tests, run under Vitest.
