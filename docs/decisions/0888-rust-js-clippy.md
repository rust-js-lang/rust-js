# 0888. `rust-js clippy` lints the app as rust-js reads it

Status: Accepted. Amends [0440](0440-a-binding-needs-no-allow.md) and
[0222](0222-editor-check.md); extends [0412](0412-rust-js-build-is-its-own-process.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): N: a lint changes nothing
a program does; what's said of it is what's true of it.

## Context

React's port runs `cargo clippy --workspace -- -D warnings`, clippy's
default lints, as a Rust project does. A plain clippy is a plain rustc
(ADR 0113): it doesn't look inside `jsx!`, and it doesn't know what JS
reads (ADR 0440). So it failed on idiomatic port code where rust-js's own
check is clean:

- `ConsolePatchingDev`'s `Logs`, a `PropertyDescriptorMap` struct given
  `Object.defineProperties`: "multiple fields are never read";
- `ReactBaseClasses`' `DeprecatedApis`, a `js::Defined` struct JS reads by
  key: "fields `is_mounted` and `replace_state` are never read";
- `DefaultPrepareStackTraceV8`'s private hook, which only
  `js::export_default!(prepare_stack_trace)` names: "never used";
- and in an app, each variable, import and component only JSX uses.

ADR 0440 decided these are rust-js's to answer, by its liveness and its
`jsx!` pass, not by a second mechanism in the source (a derive reading each
field, a macro's own use of the function), and that a plain `cargo check`
or `cargo clippy`, which runs neither, still warns. rust-js's build and
its editor check (ADR 0222) run rustc's lints only. So a port had no way
to run clippy's lints with rust-js's answers.

## Decision

**`rust-js clippy [--package <name>] [-- <lint flags>]` is the app's
`cargo clippy`**: `cargo check` of the app with rust-js as Cargo's
wrapper, each crate of the app's own linted by clippy's lints, as rust-js
reads it, and nothing compiled to JS. Its lint flags are clippy's, after
`--`, as `cargo clippy`'s: `rust-js clippy -- -D warnings`.

- **One compiler.** rust-js links clippy's crates, `clippy_lints` and
  `clippy_config` with `clippy_utils` and `declare_clippy_lint`, and
  registers their lints as clippy-driver does, reading `clippy.toml`
  (`src/clippy.rs`). The crate is checked as `--rustc` checks a binding
  crate: rust-js's tool, its `jsx!` expansion and its liveness, with
  `--cfg=clippy` and clippy-driver's options, its MIR unoptimized and
  `format_args!` unflattened. It stops after analysis, as rustc writes the
  metadata Cargo asked for; nothing is lowered, as those options change
  what lowering reads.
- **The wrapper decides** (`src/cargo.rs`). `RUST_JS_CLIPPY`, the lint
  flags as JSON, which the tooling sets as `cargo clippy` sets
  clippy-driver's `CLIPPY_ARGS`, makes each crate of the app's own
  (`RUST_JS_CRATES`) linted; a crate of crates.io's is rustc's, its types
  alone, as `cargo clippy` lints none of them. Cargo is told the crate's
  check read the variable, so new flags check it again.
- **In a target directory of its own**, `node_modules/.cache/rust-js/clippy`,
  so it never makes the build's or the editor's check run again.
- **Natively only.** Clippy's crates are dependencies of the native build
  alone, as `toml` is, by target (`cfg(not(target_os = "wasi"))`): the
  playground's compiler lints nothing, and says so.
- **Clippy's lints never see JSX's own code**: what `jsx!` expands to has
  its expansion's spans (ADR 0110), which clippy's lints skip as a macro's;
  the Rust written inside it, `{label.as_str()}`, is linted as written.

### Which clippy

clippy's crates use rustc's internal crates, so they must be the clippy the
pinned release ships. rust-clippy's `rust-1.99.0` tag is older than 1.99's
rustc: it doesn't build against it (`rustc_hir::LangItem`, a `Vec` of trait
errors). The release's `src/tools/clippy` is rust-clippy's merge
`49e2f89` plus four rust-side fixes made when it branched, and three bug
fixes backported later. rust-clippy has the first as a commit of its own,
`2bc35de`, whose tree is the release's subtree at rust-lang/rust@8dac1c68,
byte for byte; no commit of it has the backports. rust-js pins `2bc35de`:
1.99's clippy without the three backports (a loop in
`needless_borrows_for_generic_args` with `const_trait_impl`, a
`nonnull_unchecked_on_box_ptr` suggestion, `--explain`'s lookup).

## Why

- **One way for hosts, as `cargo clippy` is Cargo's**: the same flags,
  `clippy.toml` and lint levels, `#[allow(clippy::..)]` and `[lints.clippy]`,
  and no `#[allow]` for rust-js's sake.
- **rust-js's answers, not a second mechanism**: what JS reads and what JSX
  uses are the liveness's and the `jsx!` pass's, ADR 0440's, as the build
  has them. Source forms that read each field natively were the
  alternative ADR 0440 turned down, and they leave JSX's uses unseen anyway.
- **Not a second rust-js check**: the editor's check (ADR 0222) and this
  one run Cargo through rust-js by one function, `tooling/cargo.js`'s
  `lintCheck`, which, given lint flags, lints. They're two commands
  because they answer two questions: whether the build compiles, its
  refusals too, which lowering says, and what clippy finds, which needs
  options lowering can't take.

### Alternatives

- **Linting in rust-js's build and editor check**: clippy-driver's
  options change the MIR and HIR lowering reads; without them clippy's MIR
  and `format!` lints are wrong. And a build would fail on a lint.
- **A plain `cargo clippy -- -A dead_code -A unused`**: loses every true
  finding of those, and still lints nothing inside JSX.
- **clippy-driver with rust-js's liveness**: clippy-driver takes no
  providers, and doesn't expand JSX.

## Costs

- **A larger compiler build**: clippy's crates build with it, 12 s (29 s
  of CPU) of a cold build's 59 s on the M3 Max, and 330 MB more of
  `target/`; they link into each `rust-js` binary, the debug one 104 MB,
  not 59 MB. A build after an edit is as before, 3.5 s (DEVELOPMENT.md's
  slow spots).
- **A toolchain bump moves clippy's commit with it**: the rust-clippy commit
  whose tree is the new release's `src/tools/clippy` (docs/architecture.md,
  "One pinned toolchain").
- **Only the app's crates that rust-js compiles are linted**: a build script
  or a native binary of the workspace is rustc's, as in the build.

## Amends

- **0440**: its first Consequence, "a plain `cargo check` still warns", now
  has its answer for clippy: `rust-js clippy`, with rust-js's liveness and
  `jsx!` pass; a plain one still warns.
- **0222**: the editor's check is `lintCheck` without lint flags; it's
  unchanged.

## What holds it

- `test/clippy.test.ts`, "rust-js clippy says what clippy finds, and
  nothing JS uses": an app with a variable and an import only JSX uses, a
  component and a private function only `js::export_default!` names, and a
  `PropertyDescriptorMap` struct given `object::define_properties`, and one
  `collapsible_if`, which alone is said, a warning, then with `-D warnings`
  an error, as the flags changed; "rust-js clippy of clean Rust says
  nothing", which writes no JS. A plain `cargo clippy` of the first warns
  seven times.
- Mutations: `clippy-lints-unregistered` and `clippy-flags-untracked`
  (`src/clippy.rs`), `clippy-without-liveness` and `clippy-jsx-unexpanded`
  (`src/main.rs`'s `Clippy`), and `cargo-clippy-compiled` (`src/cargo.rs`).
- `test/architecture.test.ts`, "an app's Rust is compiled one way": the
  one other command is `rust-js clippy`, which compiles nothing.
- todomvc and the Next.js and Vite examples lint clean; todomvc under
  `-W clippy::pedantic` is told only of its own Rust.

## Amendment: a class `js::class::<T>()` hands JS is used (2026-10-26)

React's `ReactFiberCacheComponent` keeps a shim where the host has no
`AbortController`, a function it never exports, made only by `new
AbortControllerLocal()`:

```js
const AbortControllerLocal =
  typeof AbortController !== 'undefined' ? AbortController : function AbortControllerShim() { .. };
```

Its port's `#[rust_js::class] struct AbortControllerShim`, named only by
`js::class::<AbortControllerShim>()`, failed `rust-js clippy -- -D
warnings` either way it was written. Private, as upstream's, rustc said
its `new` was never used and its fields never read, three `dead_code`
errors, as nothing in Rust calls `new`: JS does, as `new` of the class.
`pub`, as the guide's example was, clippy's `new_without_default` asked
for a `Default` of its `pub fn new() -> Self`, and a class's `impl
Default` is refused, as its methods would be a dictionary's, not the
prototype's (ADR 0761).

**A class `js::class::<T>()` hands JS is used, as rust-js's liveness reads
it (ADR 0440)**: JS makes it with its constructor and reads what its
prototype and its instances have, so its `new`, each item of its impl
and its fields are, and its fields' structs' fields, as a struct given a
binding's are; and what their bodies use: each function, constant,
static, struct made, method called and field read they name, through
their closures, walked from them as rustc walks from what it starts at,
which never starts there (`src/lower/liveness.rs`, `used_by_classes`). So
the shim is private, as upstream's is, and lint-clean, and its JS,
`function AbortControllerShim()`, is no export, as upstream's isn't.

- **`impl Default` of a class stays refused**: the shim is private, so
  `new_without_default`, which asks it only of a `pub` type, doesn't. A
  `pub` class whose `new` takes nothing still meets it: its `Default`
  would be an object of functions the class doesn't have in JS, whose
  shape a later decision would have to say.
- **Liveness only adds**: what the walk finds is used, and nothing rustc
  found used is unused, so it can keep a warning of dead code from being
  said, never say one of code that's used. It marks what a body names, a
  field it sets too, a little more than rustc's own walk does.
- Case N, as above.

Held by `test/clippy.test.ts`, "rust-js clippy of clean Rust says
nothing", whose app adds a private class only `js::class::<T>()` names,
with a field only JS reads, a struct and a function only its `new` makes
and calls, a tuple struct it makes, and a method its method calls; and
the mutations `js-class-dead`, `js-class-uses-unwalked`,
`js-class-fields-unread`, `js-class-paths-unread`,
`js-class-methods-unseen` and `js-class-fields-read-unseen`
(`lower/liveness.ts`). The guide's shim, in "A class as a value", is
private now.
