# 1198. The corpus's main build is the binary

Status: Accepted (2026-11-24, Tu's approval of #2447's design, comment
7121587335; the lead's calls on caught panics' hooks and the names). Amends
[0088](0088-corpus.md), [0360](0360-the-app-is-a-program.md)'s amendment
"the corpus as libraries" and [1182](1182-a-cases-builds-compile-in-one-rustc-session.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): none for the harness, which is how the tests
build and run a case. One exception of the second case is listed in 0262's
Consequences: an abort's forced frames (difference 64).

## Context

The corpus built each case as a library crate wrapping it,
`include!(case); pub fn entry() { main() }`. Its runner imported the module
and called `entry`, and native Rust ran the same wrapper with its panic hook
silenced. No net ever built the program a user ships, a binary,
`--crate-type bin`, and lowering differs between the two:
- a binary's `RefCell` borrows can be quiet where a library counts them
  across a `dyn` call;
- a binary is refused what a library keeps (#2335);
- a binary starts at `$main`, std's runtime (ADR 1184).

#2445's wrong answer showed only as a binary. Running every case as one
found four more, never covered:
- #2450 and #2451, quiet `RefCell`s whose held borrow a closure asked about;
- #2452, an uncaught stack overflow hooked as a panic with exit 101 where
  Rust aborts, and its two lines lost at the deepest frame;
- a library's copy of the runtime whose panics no hook wrote.

## Decision

**Every corpus case is built as a binary, the file itself with
`--crate-type bin`. Its JS is the case's snapshot, run as a process, and
compared with the native binary of the same file run as a process.**

- **Three builds, one owner (`test/programs.ts`, `Build`).**
  - `binary` is the main build. It's every net's: corpus, metamorphic,
    tidy's stages, the matrices, fuzz, object model, rustlantis.
  - `program` is the wrapper, a crate that exports `entry`, as an app's
    crate is (ADR 0360). It's compiled for every case and run only where its
    JS isn't the binary's past what the wrapper adds (`asBinary`: comments,
    blank lines, `$main`, `entry`, `export`). The compiler's output decides;
    there's no directive. Where it isn't, its JS is a second snapshot,
    `<case>.program.js`, and where it is, there's none (amended, below).
  - `library` is that crate with `--library`, as before (`//@
    library-differs`). It's compared with the program build.
  - `program` and `library` share one worker session (ADR 1182); the
    binary, another crate, gets its own beside it. The wrapper crate is
    named for the case, as the binary is.
- **A run is a process (`runProcess`).** Its outcome is `{ exit }`, its
  status: 0, 101 for a panic, `process::exit`'s code, 134 for an abort
  (natively a SIGABRT). It runs in an empty directory of its own, with
  `RUST_BACKTRACE` unset.
  - Natively the case is `rustc`'s binary of the file, so the crate name,
    `file!()` and `module_path!()` are the JS's. The old silenced-hook
    wrapper is gone.
  - A program build runs under `test/corpus-run.ts`, which runs its `entry`
    by the runtime's `$main`, as a binary's `main` is run.
  - The outcome-file runner, `asMain` and the oracle's `{ panic, at }` are
    gone. A panic's message and place are its hook's text on stderr, and a
    directive's message is checked there (`hookSays`).
- **What's compared of stderr (`compared`).** The hook text of the panic
  that ends `main` is compared whole: the last one, for a run that failed.
  The hooks of panics caught before it, and of one a drop's panic aborted,
  are left out, as the corpus never compared them (the lead's call;
  #2453 compares them).
  - Of an abort, native std forces a `stack backtrace:` of frame addresses
    for a panic that can't unwind. That block is left out: JS has no Rust
    frames (Tu's approval, difference 64).
  - The JS now writes std's own hook of that nounwind panic, at
    `core::panicking`'s line (`$stdSites.panicNounwind`,
    `panicNounwindNobacktrace`), so the rest of an abort's stderr compares
    byte for byte.
  - A case `located-otherwise` compares hooks without places (`unplaced`).
- **Cases that read their outcome file's path**, the fs cases and
  `env_args`, now name files in the directory they run in, and see a
  binary's arguments: its path alone.

## Why

- **It's what users run.** A binary is the build an app ships. A net that
  never runs it can't see what differs there, and #2445 and #2450–#2452
  were each such a difference.
- **One build per question.** The binary answers "does it run as Rust
  does". The program build answers it for an app's crate only where its JS
  differs. The library build answers it for a crate another crate uses.
- **A process is the honest oracle.** Status, stdout and stderr are what a
  terminal shows. The in-process `entry` call hid the hook, `$main`'s
  handling of what reaches it, and a binary's exit.

## Alternatives

- **The wrapper stays the main build, with a binary run where its JS
  differs** (#2447's option 2). Mutations naming a corpus case would still
  judge the wrapper, which isn't what ships.
- **Compare every caught panic's hook now** (A1). It's stricter. It turns
  the ADR 0090 usize width, and caught-panic places never compared, into
  about 15 failures to triage first. It's filed as #2453.
- **A `//@ library` directive for a case whose point is its exports.** No
  case needs one: export cases live in their own files. It waits for the
  first such case.

## Consequences

- Every snapshot moved once, in one bless (2,879 files):
  - 2,834 by the wrapper's own changes alone: the crate root's comments, no
    `export`, no `entry`, `$main` imported and called;
  - 19 by a type id's crate name and hash, `lib::AppError#..` now
    `app_errors::AppError#..`;
  - 21 by lowering: quiet `RefCell` borrows, unwind forms, and imports
    placed after a hoisted function;
  - 5 by their cases' edits.
- `getter-reads` stays a library's calls: its cases are exports JS calls,
  not programs.
- The rustc suite (`scripts/rustc-suite.ts`) still runs its tests' wrapper
  under the outcome-file contract's successor. Moving it to binaries is its
  own change.
- Measured on the 12-core Mac mini, the whole corpus in 6 parts:
  - warm, every compile kept: 5–7 s, as before;
  - every compile run, as a compiler change makes it
    (`RUST_JS_COMPILE_CACHE_MB=0`): 58–59 s against 41–42 s, +17 s, all
    of it the binary's compile, a crate of its own that can't share the
    other builds' session (a slow spot, #2455);
  - every run re-verified: 129 s before, 155 s after;
  - the first run after the change, about 2,900 native binaries built and
    first checked by macOS: 335 s, once.

## Amendment: a program's JS is a snapshot too (2026-11-24)

A decision the exported build alone shows, a cell counted where `entry`
hands the crate to a caller, a copy kept, a nullable exit, was held by no
snapshot once the binary's became the case's: the old snapshot was the
wrapper's. The mutation `held-field-read-unsettled` survived so on main
(worker-02 agent's read, #2447 comment 7127662433): its counted `lines` shows in
`scope_struct_of_type_parameters`'s program build alone.

- A case whose program JS isn't its binary's past what the wrapper adds
  (`asBinary`) keeps it beside it, `<case>.program.js`, its header naming
  the case, as `<case>.js` does. The case fails where it's missing, where
  it isn't the program's JS, or where one is kept of a program whose JS is
  its binary's. `bun run bless` writes or removes it.
- A run of snapshots alone (`RUST_JS_SNAPSHOTS=only`, a mutation's judge,
  #864) compiles the program build beside the binary, so a mutant that
  moves only a program's JS is judged by it too.
- 40 cases of 2,970 keep one: `dyn Any`'s and `dyn Error`'s type ids,
  generic and `dyn` cells, io errors, drops. `held-field-read-unsettled`
  is caught by `scope_struct_of_type_parameters`'s, whose program's JS
  differs with the mutant.
