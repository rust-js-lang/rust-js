# 1109. A small Crater: the crate corpus's own `#[test]`s, each as native or not

Status: Accepted (#1087; the net the user's queue asked for, 2026-11-09).
Extends [1067](1067-regression-nets-the-nightly-casts.md), whose crates it
runs, and [0916](0916-rust-js-test-runs-a-crates-tests-as-js.md), whose
`rust-js test` runs them.

Case ([0262](0262-when-rust-and-js-disagree.md)): none decided here. This
is a net: each wrong answer it finds is its own issue, decided there.

## Context

Crater builds and tests every crate on crates.io before each rustc
release. rust-js's crate corpus compiles 22 crates and runs a probe of
each, but a crate's own tests exercise its code the way its authors meant,
and nothing ran them as JS.

## Decision

**`bun scripts/crater.ts [--check] [name ...]`**: each crate of
docs/crate-corpus.md, the one list, at its exact release.

- **Natively**: its source, as crates.io packs it, copied out of Cargo's
  registry, and `cargo test --lib`, its own lock where it packs one.
- **As JS**: a second copy, its integration tests left out (`autotests =
  false`, no `[[test]]`), and `rust-js test`, its unit cleaned first: a
  build that stopped may have left it as done with no tests written.
- **Each test compared**: one that passes natively and fails or doesn't
  run as JS is a wrong answer, a P0 reduced and filed, unless
  `scripts/crater-decided.txt` lists it with the guide's reason: num-traits'
  `copysign` and `total_cmp` of a NaN's sign and payload, which the guide
  says aren't kept.
- **A refused test is cut, not the crate**: every refusal names its
  `file:line`, so the JS's copy gets `#[cfg(any())]` above the `#[test]`
  it's in, or above a function only tests build, one in a `#[cfg(test)]`
  module or file, whose callers are refused and cut the next round; the
  rest is built again, up to twelve rounds. Each test is then said by
  itself: as native, a wrong answer, or refused and why. A refusal in the
  library's own code refuses the crate, said with its first refusals.
- **Kept by its release and the compiler's build**, in
  `target/crater/kept/`; `target/crater.json` holds the night's verdicts.
- **`--check`** fails on a wrong answer no decision lists: the nightly's.

The user chose per-test verdicts over a crate's whole (2026-11-09), first
as a compiler switch beside `--js-made-all`. A cut of the source does it
with no compiler surface: a switch would have to tie each refusal to a
test, and several are made before any function is lowered, an impl's
evidence among them. Decided with the user, through m6-react.

## Consequences

- Its first run, 22 crates, 100 s with its builds kept (2.35 GB at its
  peak): 212 tests as native, of heck (111), bitflags (51 of 54, 3
  refused), num-traits (44 of 46, its other 2 decided), either (4 of 8, 4
  refused: `{:e}`, `io::Cursor`, `stdin`) and arrayvec (2); hex's 14 all
  refused, on pretty_assertions' `assert_eq!`. No wrong answer. The rest
  are listed in #1087 with what stops each: a library refused in its test
  build, its default features on (chrono's user `FromPrimitive`, url's
  `form_urlencoded::Target`, time's `SmartDisplay`, once_cell's
  `thread::current`, uuid's `wasm_bindgen_test` of wasm32, anyhow's
  `as_str()` of a parameter, #1091), a dev-dependency that doesn't build
  for wasm32 (getrandom's: indexmap, hashbrown, regex, base64), a graph
  with two releases of the crate (itertools), and crates with integration
  tests alone (strum, thiserror, semver).
- Built cold, chrono's dev-dependency `windows_bindgen` takes rust-js 9 GB
  (#1096); every other crate's compile is under 0.71 GB.
- `test/crater.test.ts` holds its reading of Cargo's, Node's and rust-js's
  output and its cuts; the net itself runs each night.

## Amendment: what a cfg builds only for tests, or never on wasm32 (2026-11-10, #1153)

chrono's own tests, with its default features, had its library refused:
its `tz_data.rs`, `#[cfg(all(any(target_os = "android", target_env =
"ohos", test), feature = "clock"))]`, is built on wasm32 only because of
`test`, and reads files through `io::Read`, which rust-js refuses (#1156).

- **A module whose cfg holds on wasm32 only because of `test` is
  test-only**, as `#[cfg(test)]`'s is: a refusal in it cuts its functions
  and the tests that reach them, never the library (`onlyForTests`). Its
  cfg is evaluated of rustc's own cfgs of the target, `rustc --print cfg
  --target wasm32-unknown-unknown` (`targetCfgs`), the one source, and of
  `test`. A `feature = ".."` is neither true nor false, which the Crater
  can't know: test-only means false without `test` whatever features are
  on.
- **A native test of a module a cfg leaves off wasm32 in a test build**,
  chrono's `#[cfg(unix)] mod tz_info;`, is no test of rust-js's target: it
  is counted in a column of its own, "not of wasm32" (`offTarget`), never as
  one that differs, nor as passed.
- Cut tests stay a column of their own, "refused".

chrono 0.4.45 with its default features, its row's now: 240 of its 272
tests as native, 12 refused, 20 not of wasm32, none differing. Held by
`test/crater.test.ts`: chrono's tz_data cfg test-only and others not, and
`tz_info`'s test of no target's.

## Amendment: integration tests too, each file by itself (2026-11-10, #1087)

A crate's integration tests, `tests/*.rs`, are its authors' tests as its
library's are, and semver's, time's, strum's and thiserror's are all
they have. The Crater runs them now.

- **Natively, each test binary by itself**: `cargo test --lib --tests
  --no-run` builds them (`--lib` alone where those don't build), and each
  is run on its own, its outcomes its target's, `tests/zip.rs::t`
  (`executables`). Read off `cargo test`'s combined output, a test's
  `Running` line, its stderr, and its tests' lines, its stdout, weren't
  kept in order by the pipe, and itertools' `quick.rs` tests were counted
  as the next file's.
- **As JS, each integration test's file by itself**, its library as
  `rust-js build` compiles it, none of the library's tests
  (`oneIntegrationTest`: its `[[test]]` table alone, `[lib] test = false`,
  no harness-less test), so each refusal is its own; a refusal in the
  library its test uses refuses that file's tests, never the crate. A
  refusal in `tests/` cuts the test it's in, by the module path of that
  test's crate (`testModulePath`). A file whose own `#![cfg(unix)]` leaves
  it off wasm32, chrono's `dateutils.rs`, is "not of wasm32".
- **Each crate's JS builds in a target of its own**, `target/crater/js/<name>`,
  gone once it's asked: crates built at once in one target raced on a
  dependency both build, num-traits' metadata of one build read as the
  other's, and chrono's whole run failed.
- **getrandom's WebAssembly feature, of each release its lock has, only
  where a build fails on getrandom** (`forTheJs`, `built`): `js` of 0.2,
  `wasm_js` of 0.3 and 0.4. Turned on everywhere, it put getrandom's
  backend in graphs rust-js's own wasm-bindgen family then didn't serve
  (ADR 1112), and chrono's `js_sys::Date` was refused.
- **A timeout ends the process group** it started, `rust-js test`'s
  runner and its JS among it.
- Its verdicts are kept by how it asks too (`asking`, 2).

The night's 22 crates, before and after, on the 12-core M6 (one wall
time each; the kept verdicts cleared):

| | before | after |
|---|---|---|
| crates asked (tests compared) | 9, and chrono alone | 12 |
| native tests passing, of crates asked | 374, and chrono's 272 | 1,523 |
| as native | 335, and chrono's 242 | 674 |
| refused (cut) | 21, and chrono's 10 | 483 |
| not of wasm32 | 20 (chrono) | 21 |
| decided (num-traits' NaN sign and payload) | 2 | 2 |
| wrong answers | 0 | 1, itertools `tests/zip.rs::test_zip_longest_size_hint`: `Cycle::size_hint`, filed (P0) |
| wall time | 107 s (chrono's run raced) | 20 min 22 s |

New: thiserror 38 of 53 (its tests are integration tests alone), semver
11 of 34, num-traits 60 of 62 (16 integration), arrayvec 2 of 43 and
itertools 37 of 408 (most refused, on the way to being asked), and
anyhow's, url's and base64's library refusals now name 71, 67 and 192
native tests unrun. The wall time is semver's one integration file, whose
5.6 MB identifier's byte views run past the 20-minute `rust-js test`
timeout, alone after the rest are done: a slow spot (DEVELOPMENT.md).
(Amended, #1184: it wasn't the byte views but `as_bytes()` encoding the
whole text each step of semver's `identifier`, quadratic; with a long
text's facts kept (ADR 0126, amended) `test_new` runs in 0.9 s, semver is
14 of 34 as native, its `test_identifier.rs` asked, and the Crater's wall
time 2 min 9 s on the 12-core M6 at a load of 4–7, 2026-11-10.)

## Amendment: a refusal at run time is its own column (2026-11-10, #1192)

A test whose JS failed with rust-js's refusal at run time, an error whose
message is "rust-js does not support ..", was counted WRONG with the
wrong answers: getrandom's `test_diff_u32` and `test_diff_u64` (#1191) hid
any real one among them and sent the reader hunting. `jsOutcomes` reads
each failure's error that Node's runner says again after the run, and a
test whose error is a refusal is `refused at run time: <message>`; a
crate's line counts them in a column of their own beside the refused,
`2 refused at run time: <test> (<message>)`, and `--check` fails on the
WRONG alone. Each such refusal is still an issue of its own, a refusal to
make a compile error where it can be. `test/crater.test.ts` holds Node's
runner's own words of a refused test and a failing one.

## Amendment: the top 100 crates by downloads, and what stops them ranked (2026-11-11, #1174)

The Crater's 22 crates were the ones the corpus had picked by hand. #1174
asked for the crates an app meets first: crates.io's most downloaded.

- **The list stays one.** docs/crate-corpus.md's second table, the 77
  crates of crates.io's 300 most downloaded on 2026-11-10 whose library
  builds for `wasm32-unknown-unknown` with its default features and whose
  package has `#[test]`s of its own, no proc-macro crate and none of the
  wasm-bindgen family rust-js serves itself (ADR 1112): with the first
  table's 23, the first 100. Each row at its exact release, its build
  metadata too, `toml 1.2.1+spec-1.1.0`, its graph locked as every row's is.
  A push's `crate-corpus.ts --check --sample` asks every row of the first
  table and one in 8 of the second, a hash of the commit and the crate
  picking them (`sampledIn`): the second table's graphs refused somewhere
  are built again every run, 94 s of the 100 rows' warm run where the first
  table takes 6 s; sampled, 8 s warm and 24 s cold. The nightly asks every
  row; `crateProjectsCap` is 4 GiB, as 100 graphs are 1.2 GiB built once.
- **What stops them, ranked** (`ranked`): each refusal's reason, its place
  taken off, by the crates it stops and then their native tests, a test cut
  for it, refused at run time with it, or every test of a library it
  refuses; a crate not asked is ranked by why. `target/crater-refusals.json`
  and the night's `crater-refusals` artifact, so ecosystem work is chosen by
  need.
- **The nightly in four shards** (`--shard=i/4 --out`, `--merge`), every
  fourth crate a machine, as the rustc suite's: 100 crates are about 8,000
  crate-seconds on the Mac, two at a time on a runner. `crater-report` says
  them as one run and fails a night a shard is missing.
- **The list of differences says three kinds**, each a line under its
  reason: a decision (the guide's), a wrong answer known under its open
  issue, `#1264 ..`, shown as `WRONG known`, and a test cut before its JS
  is built, `cut: #1269 ..`, that would hang a shard. `--check` fails where
  a listed difference runs as native, so the list only shrinks.
- **Cuts that real crates' tests needed:** an impl only tests build, cut
  as a function is (base64's `impl Distribution for Standard`); of a `use`
  of several names only tests build, the one unresolved taken off its line,
  not the `use` (base64's `use crate::{alphabet, tests::random_config}`); a
  function only tests reach cut with the tests a macro made of it, rstest's
  `f::case_1`, which were read as not run, a wrong answer that wasn't; the
  JS build's lints at most warnings, as a cut is the Crater's edit and a
  lint its author denies refused it (base64's `unused_import_braces`); 40
  rounds, not 12; a dev-dependency a feature names kept (tinyvec's
  `criterion`); a cut test that failed natively not counted as refused
  (proc-macro2's "-4 of 57"); a failed build said by its first error, not
  Cargo's last words; getrandom's feature asked of the crates Cargo couldn't
  compile, not of a message's words; and a Crater stopped by a signal ends
  its process groups, which had gone on building.
- **`target/crater/` a store of its own** (`craterCap`, 6 GiB, `bun run disk`):
  the 100 crates' native test builds are 13 GiB in one target, which the
  disk's budget can't hold (ADR 1118); past the cap they go, its kept
  verdicts never, and a run after builds them again.
- **What it reads and cuts of a crate's source** is a module of its own,
  `scripts/crater-cuts.ts`, as `scripts/crater.ts` reached 1,350 lines.

The 100 crates on the Mac (2026-11-11, at aa155116c; the kept verdicts
cleared, the native test builds kept):

| | before (23 crates) | after (100 crates) |
|---|---|---|
| crates asked (tests compared) | 13 | 35 |
| native tests passing, of crates asked | 1,190 | 3,750 |
| as native | 694 | 2,619 |
| refused (cut) | 471 | 1,107 |
| refused at run time | 2 | 0 |
| not of wasm32 | 21 | 21 |
| decided (num-traits' NaN sign and payload) | 2 | 2 |
| wrong answers | 0 | 12 tests of strsim, #1264 (P0), fixed in 7bcf6b29a; getrandom's `test_diff_u64`, its `MaybeUninit<u64>` written through a byte view since d5c8e881b, #1279 (P1, listed) |
| crates whose library is refused (native tests unrun) | 5 (342) | 35 (2,663) |
| crates not asked | 5 | 30 |
| wall time | 167 s | 594 s; 1,360 s with every native build cold and typenum's hang (#1269) |

Not asked, by why: 10 packages with no tests (serde, strum, time, nix,
futures and others); 9 whose dev-dependencies don't build for wasm32 (mio's
hyper, h2 and tokio-util; wait-timeout's winnow, toml_edit and nom;
snapbox's clap_builder; net2's futures-util; criterion's tinyvec), and
rand_chacha's getrandom, whose package has no lock to say its release; 5
whose tests don't build natively (tokio, syn, http, rustls-webpki,
rust_decimal); hashbrown, indexmap and memchr, which rust-js reads as rustc
builds them (`READ_AS_RUSTC_BUILDS_THEM`), so no test of theirs is ever
rust-js's; base64, its 40 rounds spent, its first cut for a user `rand`
`Distribution`; and zerocopy, a crash of rust-js's (`back_stepped`).

Found: #1264, strsim's `jaro`, `zip` in generic code of a `&I` whose
`IntoIterator` is the crate's zipping the value itself; #1269, typenum's
`Binary` of a type 32 deep, oxc_formatter's layout doubling with each level
of nested calls. The refusals that stop the most crates: `LocalKey::try_with`
(4 crates, 132 tests), `Into::into` and `TryInto::try_into` called
(4 crates), a pointer read as another type's (3), `std::process::Command`
(3), and `&mut &mut [T]` values (bytes, miniz_oxide, httparse: 1,050 tests).

## Amendment: each JS build starts from the crate's own lock (2026-11-11, #1400)

The JS's builds run without `--locked`, as a cut changes the source, and
each integration test's manifest names only the dev-dependencies it uses.
Cargo pruned the rest from the lock, and the next build, naming them again,
resolved them anew at their newest: itertools' quickcheck graph ran
ppv-lite86 0.2.21 and zerocopy 0.8.63 as JS, where its packed lock, which
native Rust ran, has ppv-lite86 0.2.17 and no zerocopy, so a refusal was
reported of a crate native Rust never built. Each JS build now starts from
the crate's own lock again, so a dependency named again comes back at the
release native Rust ran.

## Amendment: which tests a cut took, read of the expanded crate (2026-11-22, #1420)

itertools' `tests/test_std.rs` made 16 tests with a macro invoked where an
item goes, `generic_test!(k_smallest_sort, u8, ..)`, through paste and
`quickcheck!`. Its helper `k_smallest_sort` was refused and cut, and the
next round's error was at the invocation. No cut covered an invocation,
so all 86 tests of the file counted as refused, with "cannot find
function `k_smallest_sort`" as their reason. The tests an invocation
makes have names no scan of the source can give, `k_smallest_sort_u8`.
Matching names by the macro's argument could take a test it didn't make,
`k_smallest_sort_alone`, and hide that test's refusal. `cascaded` made
that kind of guess for every crate: a native test that passed and that
the JS didn't run was put down to the nearest cut. That held only for a
run with no dead file, and it would also have hidden a test that was
built and then lost.

- **The expanded crate is the one authority on which tests a cut copy
  holds** (`expanded`, `testsOfExpanded`). Each build's copy, its library's
  or one integration test's, is expanded natively with `cargo rustc
  --profile test -- -Zunpretty=expanded`, and its tests are read from
  rustc's test harness, `#[rustc_test_marker = "path::name"]`, the
  macro-made tests included. rustc writes the whole expansion even when
  some names don't resolve because a helper was cut. A long path's marker
  is printed across two lines. Every marker must be read, or the copy is
  not asked: a marker missed would make its test look taken by a cut. The
  first whole run missed httparse's two and chrono's 20 for this reason. A native test that
  passed, ran as no JS and no cut names is one a cut took only when its
  build's copy no longer holds it. It is then refused for the cut nearest
  it by module path, as before. A test the copy still holds and didn't
  run is a difference, and so is a test of a build that wasn't expanded:
  nothing is guessed.
- **A macro invoked where an item goes is an item** (`items`, kind
  `macro`, by the macro's path). It is cut as a function is, where only
  tests build it, when no function or impl is around the error. A
  `macro_rules!` and an invocation in a function's body are not items,
  and a function inside an invocation, `quickcheck! { fn prop .. }`, is
  still cut by itself. The tests an invocation's cut makes are known
  exactly (`testsMadeBy`): those the expansion holds without its cut and
  loses with that cut alone.
- **A cut for a function an earlier round cut is said with that
  function's reason** (`followed`). rustc's "cannot find" of a function
  of `items` is replaced by the reason of the cut function nearest it by
  module path, which is the chain's root for a chain of helpers. The
  round's own error is kept after `; its cut left: `, which the ranking
  leaves off (`reasonOf`), so the ranking names what refused.
- `-Zunpretty` is rustc's unstable printing. `RUSTC_BOOTSTRAP=1` is set
  for that one command alone: the expansion is native dev tooling and is
  never part of a build of rust-js's. It has a target of its own,
  `target/crater/expand`, because the setting rebuilds what the native
  build built. A copy whose expansion can't be read is not asked, and the
  verdict says why. `asking` is 16.

All 100 crates on the Mac mini (2026-11-22, compiler at cc5e1deb0), old
code against new, kept verdicts cleared: each crate's counts are the same
(as native, cut, not of wasm32, at run time, 0 differences).
byteorder's 390 cut tests, 388 of them made by macros, are still cut,
read now from its expansion. itertools stays at 130 of 415. Its
`tests/test_std.rs` gets past the invocation, and its 86 tests stop at the
library's own refusal as that file uses it: "stepping `I`"
(src/powerset.rs:65), now 143 tests, not "cannot find function
`k_smallest_sort`". The run took 15 min 34 s, against 18 min 36 s for the
old code's.

## Amendment: a JS run past its budget is asked test by test, and the Crater is crater.yml's (#2460)

Nightly 45309220022's report listed 149 of itertools' tests as wrong
answers, all but one "not run": quick.rs's 92 and specializations.rs's 56.
Once getrandom ran as JS (f19e64f17), quickcheck ran each property's 100
random cases for real, where each had panicked at `thread_rng` at once.
Each file's `rust-js test` then ran past its 1,200 s. Its group was
killed, its first property still running. The only lines the run printed
were the 6 library tests every integration build runs too, so
`cutUntilBuilt` took those as the run's outcomes, the timeout was dropped,
and every test of the file was "not run". itertools' line came 38 min after
the crate before it, two budgets, where on 11-23 it came 11 s after.

- **Each test it didn't say runs again alone** (`afterTimeout`): each test
  of a test file the run wrote, in its target, since it started, that
  printed nothing, its own process and 120 s, as a dead file's tests
  already were (`afterDeaths`, which shares `aloneEach`). One that ends
  is its own outcome. One still running at its budget is `timed out:
  after 120 s, run alone`, a difference until it's shown to end, since a
  hang where native ends is a wrong answer. Node's own `--test-timeout`
  can't do this: it never stops a test whose JS runs without awaiting, as
  a compiled test's does (a 3 s busy loop passed a 1 s timeout).
- **Both runs are given quickcheck's cases, 20 a property** (`bothRuns`,
  `QUICKCHECK_TESTS`), natively and as JS alike. Both read it through
  `std::env::var`, which rust-js reads of the JS process's environment.
  The comparison stays one program's, and its JS run is a fifth of the
  work.
- **crater.yml is the Crater's one definition**, started by hand for the
  crates named (`gh workflow run crater.yml -f crates=itertools`) and
  called by the Nightly for every crate in four shards. The Nightly copied
  its steps, so a fix of the Crater could be proved on CI only by the
  next night. test/ci.test.ts holds that only crater.yml runs
  scripts/crater.ts.

`asking` is 17.
