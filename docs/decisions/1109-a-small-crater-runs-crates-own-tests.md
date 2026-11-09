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
