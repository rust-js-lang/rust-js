# 0360. The app's crate is a program: `RUST_JS_APP`, recorded for Cargo

Status: Accepted. Amends [0100](0100-separate-crates.md) and
[0101](0101-cargo-workspace-wrapper.md). Amended by
[0914](0914-a-librarys-generic-function-is-its-programs.md): a library's
generic function is its program's.

## Context

Cargo builds each crate with `--library`, the app's too (ADR 0101): a crate
built as the app would be fresh when another build used it, and Cargo
can't say which crate it was asked for, as it drops `CARGO_PRIMARY_PACKAGE`
from what it tracks. A library assumes another crate may use what it
exports (ADR 0100), so what only a library can't do the app couldn't do
either: of the corpus's programs, 10 compiled alone were refused built by
Cargo, seven of them a counted `Rc` "another crate may share". A Next.js
page is such an app.

## Decision

**The tooling names the app, `RUST_JS_APP`: the package it builds, or its
manifest's own. rust-js compiles that package as a program, and records
`RUST_JS_APP` in the dep-info, as rustc records what `env!` reads, so
Cargo builds a crate again when it changes.**

- `checkCargo` sets it to the package it's asked for, or the manifest's
  package; the editor's check the same, so its errors are the build's. A
  workspace's root without a package names none: every crate a library.
- In one build, nothing uses the app: Cargo builds it and its dependencies.
  A build of another package that uses it has another `RUST_JS_APP`, and
  Cargo builds the app again, a library.
- Its manifest has no `library`; the tooling names its crate as Cargo does.
- A Cargo build without the tooling, no `RUST_JS_APP`, is what it was.

## Consequences

- The workspace test builds the app, a crate that uses it, the app again,
  and the whole workspace: program, library, program, library.
- The react.dev port: one line, `formatStr(consoleData.data)`, as react.dev
  writes it, where a library copied the array.

## Amendment: the corpus as libraries

Each corpus program is compiled as a library too and run against native
Rust. What a library refuses that a program doesn't is its directive,
`//@ library-refused: <text>`, asserted, and an error once it compiles:
seven counted `Rc`s another crate may share (ADR 0320), and two values
with destructors. A trait's default constant a library computed where no
consumer reads it, an error rustc never gave, is now its initializer
(ADR 0176).

## Amendment: a crate another uses is a library, whichever is built

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same program
run either way; only which JS a crate is written as changes.

React's port built `--package shared` wrote `shared/enqueueTask.js` as a
program's, then `--package react-isomorphic`, which uses `shared`, wrote
it again as a library's: its `Box<dyn Fn()>` parameter dropped what it
held (ADR 0521), `$catchUnwind`, `$dispose` and `$dropping` in a `try`
React has none of. A crate's JS is written once, beside its Rust, so which
of its two forms is there was which package was built last: two answers
to whether a crate is a library, the package asked for and the crates
built.

**The app is the package the tooling builds, or its manifest's own, where
no crate of the workspace uses it**, as Cargo's metadata resolves the
workspace; one another crate uses is a library whichever package is
built, so its JS is the same built alone as through what uses it.
`cargoBuild` answers it, for the build and the editor's check alike.

- The workspace test's app, used by a crate, is a library built alone,
  through the crate that uses it, and alone again, the same bytes each
  time; the crate that uses it, which nothing does, is a program.
- A crate a native oracle of the workspace uses, as the workspace test's
  `check` calls `frontend::main`, is a library, and prints what native
  Rust does as one.
- Cargo builds a used crate again where `RUST_JS_APP` changes, which it
  records; the JS it writes is the same.

## Amendment: a library's function takes what its own crate's calls give it ([0914](0914-a-librarys-generic-function-is-its-programs.md))

A crate used by another is a library, its JS the same whichever package
is built (above), but its JS changed the day another crate started using
it: React's `createContext(defaultValue, TCopy)` and `useState(
initialState, dropS)`, where they had been React's own. Its generic
functions now take what its own crate's calls give them, as a program's
do, so they're the same with a consumer as without, and a consumer that
gives one more compiles its own. What a library still writes that its
program wouldn't, its consumers' trait impls and copies of what they may
change in place, ADR 0914 lists.

## Amendment: each push builds as libraries the cases whose JS differs

Case ([0262](0262-when-rust-and-js-disagree.md)): none; this decides which
corpus cases each push builds as libraries, not what the JS does.

Building every corpus case as a library too was about 30% of the corpus's
CPU, and the corpus is the longest part of each push's check (ADR 1030).
Of 200 cases sampled, 181 were the same JS as a library as as a program,
byte for byte: running that JS a second time compares nothing new.

**A case whose JS as a library isn't its program's says so, `//@
library-differs`. Each push's corpus, and `check:fast`'s
(`RUST_JS_LIBRARIES=differing`), builds only those cases, and any
`library-refused` one, as libraries. Every other run builds every case:
the nightly's, a mutation's, and one by hand.**

- A run that builds a case as a library compares the two JS. A difference
  the case doesn't state, or a stated one that's gone, is an error;
  `bun run bless` writes or removes the directive.
- The library build is run against native Rust only where its JS differs:
  the same JS was run as the program.
- 120 of the corpus's 1,319 cases are marked: generic trait methods a
  consumer may give a destructor, `dyn` and `Any` values, type facts.
- A change that makes a case's library JS differ where it didn't is found
  by the nightly, up to a day later, instead of by its push. Each push still
  checks the marked cases, and `test/crates.test.ts` still checks libraries
  and their consumers.
- A local run of the 1,319 cases went from 47 s to 26 s.
