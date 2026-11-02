# 1067. Regression nets the nightly casts

Status: Accepted.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; this decides what
is checked and when, not what the JS does.

## Context

On 2026-11-02 four kinds of regression reached `main` and stayed there
unseen:

- **crates.io's crates.** 8635ec3e2 and c3ab99271 made chrono and
  rust_decimal refused (#467, #468), where docs/crate-corpus.md said they
  ran. No job ran `scripts/crate-corpus.ts`. Its first run with `--check`
  found five more worse than their rows: itertools, base64, either,
  smallvec and arrayvec (#473–#476, #468).
- **React's port.** Changes to bindings and output broke react-in-rust's
  build or changed its JS (rd540, rd617, rd623), and nobody knew until its
  porter regenerated hours later. a3884461e typed webapi's constants as
  WebIDL's `u16` and `u32`, and react-in-rust's main stopped compiling.
- **A decision read back from the JS.** 4bfa4dea9 decided whether `*r = v`
  writes a place by asking whether the JS target is a member or an index,
  in place of MIR's `refs`. A `&mut` read from where one is kept,
  `result._0`, is shaped the same, so the write replaced the slot (#466,
  P0).
- **Drops.** #463 and #465 were found within a day by the nightly's
  generated programs. That's soon enough.

The loop must stay as fast as it is. `check:fast` and each push's check
take on nothing broad.

## Decision

**What a day's delay costs little to find is the nightly's. Each push
gets only what's cheap and narrowly triggered.**

- **The crate corpus, each night** (`crates`): `bun scripts/crate-corpus.ts
  --check`. It fails when a crate does worse than its row in
  docs/crate-corpus.md. A row may say `runs (refused: #467)`: an open issue
  says the crate does worse now. That holds while the crate does exactly
  that, so the job stays sensitive to every other crate, and it fails once
  the crate does better, so the mark goes with its fix. Each crate measured
  needs a row.
- **React's port, each night** (`port`): `bun scripts/react-in-rust.ts
  regen` fetches react-in-rust at the commit `ports/react-in-rust.pin`
  names, beside a `rust-js` that is this checkout, as its Cargo.toml and
  package.json expect. It compiles each workspace crate with this compiler
  and compares the JS with what that commit committed. A compile error or
  a changed file fails the job. Its summary lists each file and the first
  lines of its diff, and the whole diff is an artifact. **Moving the pin to
  the port's commit that regenerated it is how an intended change is
  accepted.**
- **React's port, each push that changes a binding** (`react-in-rust.yml`):
  `bun scripts/react-in-rust.ts check`, a native `cargo check` of the
  port's workspace at the pin against the pushed bindings. It needs no
  compiler, and its `paths` are the binding crates the port uses
  (`builtins/`, `webapi/`, `node/`, `react/`), so a compiler push doesn't
  pay for it.
- **A red night is an issue**: the nightly's `issue` job opens `Nightly
  red: <date>`, or comments on that day's, naming each failed job with its
  link. The issue watch gives it an owner that day.
- **No lowering decision is read back from the JS's shape**:
  `test/architecture.test.ts` lists each test in lowering, outside tidy, of
  whether a JS expression is a member or an index with its parts unread,
  `js::ExprKind::Member(..)`. Each is listed by its function, with what it
  asks: a question of the JS itself, or the JS form of a place a fact has
  decided. A new one fails until it asks the fact's owner, or says why the
  shape is its question. It runs in `check:fast` already, in under a second.

## Consequences

- A crate's, or the port's, regression is an issue by the next morning,
  where it used to wait for someone to measure.
- The port's `regen` fails whenever rust-js changes React's JS, intended
  or not, until the pin moves. Each intended change costs a pin bump, and
  the nightly is red until someone makes it.
- The crate corpus asked for each crate's newest release its requirement
  allows, so a new release could turn a crate red with no change here
  (either and smallvec resolved to 1.19.0 and 1.16.4, though both failed
  at their recorded versions too). Amended below: each is pinned.
- Two of the shape tests the architecture test lists are marked "a fact's
  to ask": `cell_call`'s quiet `RefCell` in a field, and `mir_store`'s
  temporary `&mut` kept as the place it was read from. Each decides from
  the shape alone, as #466 did, and is to move to a MIR fact.

## Amendment: each crate at an exact release, its row's

A Cargo requirement of `1.15.0` is `^1.15.0`, so the corpus measured each
crate's newest release, and a red night could be crates.io's doing, not
rust-js's: either and smallvec resolved to 1.19.0 and 1.16.4 on
2026-11-02. A night's verdict must be of this checkout alone.

- **docs/crate-corpus.md's table is the corpus, its one list.** Each row
  whose verdict cell is a verdict is a crate: its first cell its name, its
  exact release and, in parentheses, its features, built with those alone,
  its default ones off: `chrono 0.4.45 (`alloc`)`. The script's own list of
  crates, which the rows had mirrored, is gone.
- **`bun scripts/crate-corpus.ts` asks Cargo for that release exactly**,
  `version = "=0.4.45"`, and refuses a row of no exact release, naming it,
  before it measures anything; a crate named on its command line with no
  row is refused too.
- **Moving a crate to a new release is an edit of its row**, measured and
  its verdict written as it's made. Each was pinned to its newest release
  on 2026-11-02, what that night measured.
- The crates a pinned crate uses still resolve to their newest compatible
  releases: pinning them too would be a lockfile for each crate, kept up
  by hand. A night red from one of those names it, and its release, in
  the refusal it prints, and its issue says which.

Case: none ([0262](0262-when-rust-and-js-disagree.md)); this decides what
is measured, not what the JS does. `test/ci.test.ts`, "the crate corpus is
its table's rows, each at an exact release", holds it.
