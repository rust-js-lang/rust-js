# 1184. A binary crate runs its `main` as std's runtime does

Status: Accepted (2026-11-22, #2292; Tu's decisions: (a) a binary only by
`--crate-type bin`, (b) panic locations made at compile time in every
crate, (c) only a binary writes std's hook's text; the lead's: `Location`
a string, std's own sites constants of the toolchain).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept; and one rare and harmless difference, decided by
the user, difference 62: std's own sites are named under `/rustc/<commit>/`
where native Rust names a local copy of std's source.

## Context

rust-js compiled every crate as a library: nothing called `main`, an
uncaught panic was the host's `Error` (exit code 1, a JS stack trace), and
a `main` that returned `Err` returned it to its caller (the guide's
difference 10). A Rust program run from a terminal shows what std's
runtime does: its exit status, `Error: ..` of an `Err`, std's panic hook's
`thread 'main' (pid) panicked at file:line:col:` before anything the panic
drops, its backtrace note once, and an `ExitCode`'s status.

Where a panic was raised can't be read from JS's stack: JSC's proper tail
calls drop frames, so a source map can't name the place on Bun (the spike
of #2292). The location must be made as the program is compiled.

## Decision

**A crate given `--crate-type bin` is a binary** (a single file without it
stays a library; `#![crate_type]` is refused); cargo routes a package's
own `bin` targets to rust-js. Its root module ends with `$main(main)`,
or of a `main` that returns a `Termination`, `$main(() => ..report..)`
(`entry_of`, `process.rs`). `$main` (runtime `main.js`):

- makes stdout and stderr write at once where Node lets them be blocking,
  runs `main`, and on a status other than `0` sets `process.exitCode` and
  ends the process at once, `process.exit`, as std's exit does, so a host's
  pending timer never runs; what stdout holds of a line left unended is
  written as it exits (ADR 1130's amendment, #2319). In a browser, which has
  no process, a panic is thrown to the page and a failing status thrown as
  an error.
- `$binary` says a binary's `main` runs: then **std's hook's text is written
  where a panic is made**, `$panic` and `$stringPanic`, before what it drops
  as it unwinds, as std's hook runs at the panic, caught or not; its
  backtrace note on the process's first panic alone. A `resume_unwind`
  writes none, as std's doesn't run its hook. A library writes none
  ([0660](0660-catch-unwind-catches-what-js-throws.md), amended).

**Every panicking helper raises its panic where Rust does**, `file:line:col`,
made at compile time (design A):

- **The caller's line**, a `#[track_caller]` std function's or the
  program's own panic: the helper takes `raisedAt`, its parameter after its
  required ones (`runtime::location_at`, the runtime's own parameter lists
  the one authority). The lowering scopes each MIR statement's and
  terminator's location, `caller_location` of a call's `fn_span`, and
  `Expr::call` gives it to every helper call made there, as an
  `ExprKind::RaisedAt` node. `js::validate` refuses a call of such a helper
  given none (`Problem::Unlocated`). A helper whose panic is optional by a
  flag is two: `$ilog` and `$checkedIlog`, `$sliceSplitAt` and
  `$checkedSliceSplitAt`.
- **std's own line**, a site in std's own code, `VecDeque::swap`'s
  `assert!(j < self.len())`: a constant of the toolchain, whatever calls
  it. `bun run std` builds a whole std (`core`, `alloc`, `std`, wasm32's,
  every body's MIR, Cargo's `-Zbuild-std` of the source's vendored crates,
  once a toolchain), walks each catalogued function's MIR
  (`compiled_std/raised.rs`, `--std-raised`) as rustc's codegen places a
  site, and writes `runtime/std_sites.js`, `$stdSites`, which a helper
  reads: `throw $panic("..", $stdSites.vecDequeSwap[1])`. A helper serving
  both kinds keeps `raisedAt`, and the lowering scopes the std site where
  std's line applies, `$drain(v, a, $stdSites.vecDrain[0], b)`
  (`std_range_site`). A site that differs by type is kept by type,
  `$stdSites.nextMultipleOf.u32`, the helper given its type,
  `$nextMultipleOf(a, b, max, "u32")`; `usize`'s is wasm32's std's, the
  width rust-js models ([0562](0562-a-pointer-is-a-place-and-an-address.md),
  amended), checked against native wasm32.

The walk's rules: a site is a call of a function that tracks its caller,
or a check MIR asserts; a call is placed by its `fn_span`; through each
scope inlined of a function that tracks its caller, at its call; a body
inlined of another function is that function's, but a closure of the
walked function's own, `Result::map`'s of one; the walk follows the
function's nested items, those it calls and the closures it makes or
gives, `with_borrow`'s `|cell| f(&cell.borrow())` that `with` calls. A
specialized impl is picked by its bound (`Owner::Bounded`,
`SpecFromIterNested`'s for a `TrustedLen`), and a type's impl over
another by that type (`Owner::TypeOver`, a thread-local of a `RefCell`).
A site the catalogue names that the walk can't find fails `bun run std`.
Each panic the runtime raises in std's code names its own body's site:
std has six `capacity overflow`s past `RawVec`'s, each its own entry.

**The runtime's marks are every copy's**: where a panic was raised,
`$raisedAt`, and a `String` payload, `$stringPayload`, are registry
symbols, `Symbol.for("rust-js: ..")`, declared in `runtime/marks.js`
alone. A bundled library and the app that uses it may each hold a copy of
the runtime, and a panic one raises is caught, downcast or hooked by the
other: a mark one copy made by `Symbol(..)` is unseen by the other, a
`String` read as a `&str`, a location `<unknown>`. So is every mark of a
value one copy makes and another reads (amended by #2333): an `Rc`'s box
beside its class instance, `$countedKey`, whose count the other copy read
as `NaN` and whose `Weak` upgraded after its last drop; an array's
iterator, `$arrayIterKey`, its `size_hint` `(0, None)`; a part of a
slice's place, `$placeOfPart`, `ptr::eq` false; and `$partsOf`,
`$cloneKey`, `$itemArray`, `$itemIndex`, whose reads threw, and
`$slicedKey`. `test/runtime-marks.test.ts` refuses any `Symbol(..)` of the
runtime, and runs each against native Rust with a library on a copy of
its own.

**A `Location` is its text**, `"file:line:col"`
([0427](0427-track-caller.md), amended): `{}` the text, `{:?}` std's
`Location { file: "..", line: N, column: N }`, its parts read from the end
(a file may hold a `:`), `==` and its order by its parts.

## Consequences

- Difference 10 is gone. Every crate's output passes a location to each
  helper that raises one, a string of its file and place: the React port's
  JS too, the user's accepted cost.
- std's own sites are named under `/rustc/<commit>/library/..` on every
  machine, as the shipped std names its files; native Rust names a site of
  a body the caller's crate compiles, a generic or inlined one's, by the
  toolchain's local source where `rust-src` is installed. So does the JS
  of std's own bodies, `@rust-js/std`'s: the `core` and `alloc` it's built
  of, of rust-src, are built with it named `/rustc/<commit>/`
  (`std_sysroot.rs`, #2343), so std/ is the same from any machine.
  Difference 62,
  the user's decision. Tests normalize native's local prefix.
- Some locations don't reach their helper yet, named `<unknown>` or the
  caller's: a registry crate's sites,
  serde_json's `IndexMut` panics and std's vendored hashbrown's (#2325);
  and a `#[track_caller]` std function given as a value, which native
  Rust names by the `Fn` trait's shim (#2335). Each is a P0.
- Held by `test/binaries.test.ts`, each row against native Rust on Node
  and Bun read as one stream, `2>&1`; and by every net that runs a
  program against native Rust (`test/programs.ts`): its native harness's
  hook records where the last panic was raised, and its outcome is
  `{ panic, at }`, std's local files named as rust-js names them
  (`shippedStd`), compared with the JS's, the `$raisedAt` it marks. A
  case whose location waits on an issue says so, `//@ located-otherwise:
  #<issue> <why>`: compared but for where, and the corpus fails once it's
  native's. Its first run found 180 differences, 165 of them a minified
  bundle's copy of the runtime, and 13 sites std raises in its own code
  the catalogue lacked; one case waits, under #2335 (#2324 placed the
  others, ADR 0427 amended). Mutations
  of each rule.
