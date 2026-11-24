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
- A site in a crate std's source vendors, a `HashMap`'s `get_disjoint_mut`
  of two keys of one entry, hashbrown's, is named as the shipped std
  names it, under its own build's registry,
  `/cargo/registry/25cdd57fae9f0462/hashbrown-0.17.1/src/raw.rs:1289:21`, a
  constant of the toolchain read of the shipped rlib by `bun run std`:
  native Rust's, no difference (#2325).
- A registry crate's own sites, serde_json's `IndexMut` panics and
  hashbrown's own map's `get_disjoint_mut`, are found in the version Cargo
  built and named under `/cargo/registry/src/`, difference 63 (ADR 0083,
  #2325); indexmap's, which tracks its caller, names the caller's.
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

## Amendment: a division is raised where its check is (2026-11-23, #2366)

A division or remainder whose check its helper throws in place of,
`$div`, `$rem`, `$bigDiv`, `$bigRem` (`queries::checked_by_division`), is
given the span of that check, the `Assert`'s, which is the division's,
`v / z`, as native Rust's panic names it; not its MIR statement's, which
of a division written to a place that's there already, `v = v / z`,
`t.0 = t.1 / d`, `*r = 9 / d`, is the whole assignment, as rustc writes
the quotient straight to the place (`queries::division_check_span`, read
by `structure.rs`'s located statements). Results change, so Rust's
behaviour is kept (ADR 0262). Found by generated programs, seeds
12467702, 12467723, 12467856 and 12467972; held by the corpus's
`division_assigned_to_a_place_raises_at_the_division.rs` and
`remainder_assigned_in_a_closure_raises_at_the_remainder.rs`, and a
mutation of each rule.

## Amendment: each panic the runtime raises passes its site (2026-11-24, #2489)

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a panic named
`<unknown>` where Rust names a line.

A collect into an `Rc<[T]>` or an `Arc<[T]>` past what a `Layout` of the
slice holds raised std's `unwrap()` of its `LayoutError` with no site, so
std's hook said `panicked at <unknown>:` where native Rust names
`allocate_for_slice`'s `unwrap()`, `rc.rs:2420:41` or `sync.rs:2387:41`.
`$stdSites` gains `rcSliceLayout` and `arcSliceLayout`, read from std's
MIR as every row is (`compiled_std/raised.rs`), and `$roomForCounted` and
`$collectCounted` take the row's site as `layout`, after `unbounded`.

So that no raise is left without one, test/architecture-panics.test.ts
counts each `$panic(..)` and `$stringPanic(..)` call's arguments in the
runtime: each passes its site, or the comment above it says why it has
none, `// no site: <why>`, of which there's one, `$allocError`, as std's
`handle_alloc_error` names no place. Held by the corpus cases
`rc_slice_collect_past_a_layout_panics_at_its_unwrap` and
`arc_slice_collect_by_generic_code_past_a_layout`, and the mutations
`layout-error-unsited`, `rc-slice-layout-at-its-unbounded` and
`rc-slice-layout-row-of-another`.

## Amendment: a run-time refusal is said as rust-js's error (2026-11-24, #2493)

What rust-js doesn't support as the program runs, `$unsupported`'s error,
reached a binary's `$main` and was thrown on, so the host reported it as an
uncaught error: Node's `std/runtime.js` line, a caret, `$Unsupported
[Error]: ..` and its stack, status 1. The rustc suite read that first line
as a crash (#2447's first CI run). `$main` now writes it as a refusal at
compile time is written, `error: rust-js does not support ..`, then ` --> `
and the first place of its stack outside the runtime's module, where the
JS asked it (`$refusedAsItRan`), and ends with status 1, after what the
streams hold is written. A host that maps the JS's source map gives the
Rust source's place. A browser's is thrown on, as before.

No output a Rust program has changes: native Rust has no such refusal
(ADR 0262: Rust's behaviour cannot be preserved, so it's refused, here
as clearly at run time as at compile time). Held by binaries.test.ts'
"a run-time refusal ends a binary" on Node and Bun, scripts/rustc-suite.ts'
`exited` reads the line (status 1 alone), and mutations
`binary-refusal-uncaught`, `binary-refusal-placed-in-the-runtime` and
`rustc-run-refusal-of-any-status`.

## Amendment: the start runs after all its module makes (2026-11-24, #2527)

`$main(main)` was placed by load order (ADR 0306), which reads only what
`main` reaches in its own module. Through an import cycle, `main` may call
another module's function that calls back into this one and reads what it
declares after `main`, such as a generic impl's cache, `const $wrapShow = new
WeakMap()`. The start ran before that `const` was made, and the JS threw
`Cannot access '$wrapShow' before initialization` where Rust printed. This
was a wrong answer, found by #2481's `observe-borrows` rewrite in
`guard_held_in_an_undo_stack.rs`, whose `main` calls its `tests` module's
functions.

The start now runs last of all its module makes as it loads.
`js::entry_last`, beside `in_load_order`, moves it past each `const`,
namespace and statement that load order left after it. It never moves the
start earlier, so a module whose start already ran last prints as it did,
and only functions, made as the load begins, follow it. This is ADR 0262's
"results change" case: Rust's behavior is preserved however the output
reads. Held by the corpus case `main_through_a_cycle_reaches_an_impl_cache`
and the mutation `entry-where-main-reads-place-it`.

## Amendment: a panic ends the process at once with its output a file (2026-11-25, #2538)

`$main` ended a failing binary at once only where both streams could block,
a pipe Node made blocking or a terminal. Elsewhere it waited for each
stream's write to call back: `process.stdout.write("", () =>
process.stderr.write("", () => process.exit(..)))`. While it waited, what
`main` had queued before it panicked ran, a promise's reaction and a
microtask, and printed where native ends at the panic. That path was taken
where either stream is a file, Node's `SyncWriteStream`, and by Bun for
every stream, as Bun gives each one `_type` `"fs"` and no handle. It was a
wrong answer, found in review (#2537's cause).

**A stream of `_type` `"fs"` writes what it's given at once, so the process
ends at once there too.** Measured, 2026-11-25, Mac mini: 2 MB written to
stdout and to stderr, then a panic, read by a slow reader, `| (sleep 1; wc
-c)`, and into files. Native, Node and Bun each wrote all of it, stdout
2,000,000 bytes and stderr 2,000,150 (native) or 2,000,146 (the JS, a
shorter process id in the hook's line), through a pipe of either stream,
of both, two files and one file, all with status 101. Only a stream that's
none of those, a socket, still waits for its write.

ADR 0262's case: results change, so Rust's behavior is kept. Held by
`test/binaries.test.ts`' "a panic ends a binary at once with its output a
file: nothing queued runs" and the mutation `file-stream-waited-for`.
