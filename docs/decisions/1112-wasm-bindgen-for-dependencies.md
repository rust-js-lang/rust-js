# 1112. rust-js's own wasm-bindgen, js-sys and web-sys, for dependencies

Status: Accepted (the user's choice of option (a) on #1121, 2026-11-10;
m6-react decided, for the user, how it's substituted and which crates it
refuses). Step 1 of #1121: the substitution and the refusals. The API
itself grows crate by crate, in amendments.

Case ([0262](0262-when-rust-and-js-disagree.md)): N for what's substituted.
Each item will be a mapping onto the JS that crates.io's binds, so a
crate's behaviour is the same. Refused where the substitution can't hold.

## Context

rust-js builds every crate for `wasm32-unknown-unknown`, so a crate takes
its wasm32 path. That path often calls wasm-bindgen, js-sys or web-sys:
chrono's `wasmbind` (on by default), uuid's `js`, getrandom's `js` and
`wasm_js`, time's, instant's (#1121's survey). rust-js can't compile them,
as their API is wasm ABI glue the wasm-bindgen CLI rewrites, so such a
crate is refused. Steering crates off that path (a cfg, features off)
gives silent wrong answers: chrono's `Local` always UTC, fastrand's fixed
seed. Per-crate backends, getrandom's `custom`, cover randomness alone.

## Decision

**rust-js ships its own `wasm-bindgen`, `js-sys` and `web-sys`, each a
folder of `tooling/compat/` in `@rust-js/build`, and an app's dependencies
use them in place of crates.io's.** The folders are the one catalog
(`compatCrates`, tooling/patch.js). Each is a binding crate (`bindings =
true`, ADR 0414): rustc checks it, and rust-js reads its metadata. Each
item it gains is a thin mapping onto the js and webapi crates' bindings,
never a second implementation; what those lack is added there first.

- **How: Cargo's `[patch.crates-io]`**, in the block of the app's
  `.cargo/config.toml` that tooling/patch.js writes, the one writer (ADR
  0412), with a line of its own. Cargo, its lockfile and rust-analyzer all
  see the substitution. Swapping sources in the compiler's rustc wrapper
  was rejected: it hides them from Cargo, and can't give them the js and
  webapi crates as dependencies without inventing `--extern`s.
- **Only while Cargo's graph has one.** Cargo warns of a patch it doesn't
  use, on every run, and writes `[[patch.unused]]` in the app's
  `Cargo.lock`. So `cargoBuild` (tooling/cargo.js) reads Cargo's metadata,
  and `settleCompat` names them where a package of the graph has one of
  their names, crates.io's or rust-js's, and drops them where none does.
  Where that changed it, the metadata is read again. The patch keeps its
  state between builds, so a steady app's build reads it once, as before.
- **Each at the newest release of crates.io's of its minor**, wasm-bindgen
  0.2.130, js-sys and web-sys 0.3.107, so every `^` requirement takes it.
  A dependency asking `^0.2.131` would miss it, so the nightly's
  `test/compat-versions.test.ts` reads crates.io's index and fails naming
  each folder's Cargo.toml and the version to bump it to.
- **A dependency that pins another version**, `wasm-bindgen = "=0.2.100"`,
  gets crates.io's from Cargo, which rust-js can't compile: refused before
  the build, naming the dependency, its pin and rust-js's version
  (`compatRefused`).
- **An app's own crate that depends on one is refused**, the patch there
  or not: an app binds JS by the js and webapi crates, the one way, and
  this layer serves crates written for wasm-bindgen alone. "The app's own"
  is a crate of a path, not a binding crate, and not as Cargo packaged it.
  `cargo package` writes the manifest it was given beside its own as
  `Cargo.toml.orig`. So every crate crates.io has, unpacked, vendored
  (`cargo vendor`), a `[patch]`'s copy, or the Crater's, has one, and no
  source an app writes does. The test is a fact of the source, not a
  setting, so there's no key an app could set to opt out. `packaged`
  (tooling/cargo.js) is the one answer, which the build asks, and the
  Crater through it.

## Consequences

- Until a crate's surface lands, a dependency that calls into one of them
  is refused as before, by rustc's errors at its calls rather than at
  wasm-bindgen's ABI. Each amendment lands the items one crate needs:
  getrandom (0.2's `js`, 0.3's `wasm_js`), uuid's `js`, chrono's
  `wasmbind` (`Local` with the real timezone offset, against native with
  `TZ` set). The `#[wasm_bindgen] extern` blocks crates write inline need
  rust-js's own `wasm-bindgen-macro`, which expands them to bindings.
- The `$fillRandom` backend planned for getrandom is dropped: getrandom's
  own `wasm_js` backend through this layer is the one way.
- Held by `test/compat.test.ts`: a git crate standing in for crates.io's
  builds through rust-js's crates; an app's own crate with wasm-bindgen is
  refused with the patch present; a path crate is refused until it has
  `Cargo.toml.orig`; with none left in the graph, the patch drops them and
  the lockfile has no word of them; a pin is refused, naming it. Also by
  `test/compat-versions.test.ts`, nightly. These are tooling's JS, which
  the mutations harness doesn't mutate.
- The guide: [Crates that use wasm-bindgen](../guide/libraries/wasm-bindgen.md).

## Amendment: only for crates it serves, and `#[wasm_bindgen]` of imports (2026-11-10, #1129, #1121 step 2)

**A regression of step 1 (#1129).** Once the patch named rust-js's
crates, a crate of the graph that used API they don't have yet failed to
type-check: rustc's E0432 in its types-alone build, which stopped the
whole build. Before, rustc built it against crates.io's js-sys and only a
call into it was refused. Seen with getrandom's dev-dependency
wasm-bindgen-futures (`js_sys::futures`). chrono with its default
`wasmbind` (`js_sys::Date`) was affected the same way.

- **The patch names rust-js's crates only where they serve every crate
  of the graph that uses one**, as `tooling/compat/covered.txt` lists it,
  by name and release, `getrandom 0.3`, owned beside the crates it's
  about. Any other graph keeps crates.io's, as before step 1, chrono's
  included, even beside getrandom: all or nothing per graph. A crate is
  added with its proof.
- **What the patch names is exactly what the graph uses**, of rust-js's
  crates, and what those use: web-sys only where a crate uses it, so
  Cargo never warns of a patch it doesn't use. rust-js's crates use one
  another by version, through the patch, as crates.io's do.
- **The binding crates they map onto**, `rust-js-builtins` and, for
  web-sys, `rust-js-webapi`, are named too, where no npm package of the
  app's has them: the package Node finds from `@rust-js/build`, or the
  checkout's.
- **Each declares crates.io's features by name**, `js-sys/std` and the
  rest, so a dependent's ask resolves. A call of what a feature would add,
  and rust-js's crate lacks, is rustc's error.
- **A pin is refused only where the patch names the crate pinned**:
  elsewhere crates.io's is what's meant.

**`#[wasm_bindgen]` of an `extern "C"` block (step 2).** rust-js's own
`wasm-bindgen-macro`, a proc macro rustc builds for the host, makes each
function a Rust function of its signature. That function calls a rust-js
binding of the JS its `js_namespace` and `js_name` name, `link_name =
"globalThis.crypto.getRandomValues"`, converting what wasm-bindgen's ABI
passes. Its `abi.rs` is the one table:

- a number, a `bool` or a `&str` is passed as it is;
- `&[u8]` is `Uint8Array.from(bytes)`;
- `&mut [u8]` is that, and what JS wrote is copied back after the call;
- `&mut [MaybeUninit<u8>]`, bytes not yet written, is `new
  Uint8Array(len)`, copied back after;
- `catch`'s `Result<T, JsValue>` is the call in a `try`, what JS threw
  its `Err`. `JsValue` is `#[repr(transparent)]`, the value itself
  (ADR 0584).

An option, a type or an item it doesn't know is a compile error naming
it, `method` or an imported JS type, never a guess. Each error keeps the
item's `#[cfg]`s, so getrandom's `atomics` import, left out by its `cfg`,
is left out with it. Such a crate isn't one `covered.txt` lists, so the
patch never applies to its graph.

wasm-bindgen gives an import a live view of wasm's memory, not a copy:
its `outgoing.rs` passes a slice as `Instruction::View`, `&mut` too, and
copies back only for exports. rust-js's one copy each way gives JS the
same bytes during the call, and the slice ends with what JS wrote. Only a
JS callee that keeps the view after the call could tell. It would then
alias Rust's memory after the `&mut` borrow ended, which Rust leaves
undefined. Case N of ADR 0262.

getrandom 0.3's backend, through it, prints:

```js
function getRandomValues(buf) {
  const uint8Array = new Uint8Array(buf.length);
  const value$1 = $try(() => globalThis.crypto.getRandomValues(uint8Array));
  const iter = $mutItems(buf).entries();
  while (true) {
    const value = $next(iter);
    if (value == null) {
      return value$1.TAG === "Err" ? { TAG: "Err", _0: value$1._0 } : value$1;
    }
    value[1].value = $f64ToInt(uint8Array[value[0]] ?? 0, 0, 255);
  }
}
```

One `Uint8Array` is made and given, and read back once. Two forms read
worse than the same code written in source, #1128 to fix:
- the loop is a `while` of `$next`, where written in source it's a `for`
  of its index, a loop an attribute proc macro writes;
- the `map_err` of what was thrown to a `JsValue`, the value itself,
  remakes the `Err`.

The real getrandom 0.3.4 with `wasm_js`, through the patch, is now
refused by its `fill`'s `core::ptr::write_bytes`, addressable memory, an
open gap of its own: types alone, as before step 1.

Held by `test/compat.test.ts`:
- a stand-in for getrandom's backend fills a slice in Node;
- `&mut [u8]` copies back what JS wrote, and `&[u8]` reaches JS as a
  `Uint8Array`;
- `catch` gives `None` for a throw;
- `method`, `String` and an imported type are compile errors naming them;
- the patch names js-sys, wasm-bindgen, its macro and `rust-js-builtins`,
  not web-sys;
- a graph with a stand-in for chrono, alone or beside getrandom, builds
  of crates.io's, the patch not written.

## Rejected: rust-js's crates over crates.io's, re-exported whole (2026-11-10)

(A), the patch only for a graph whose every crate that uses one is
covered, is all or nothing per graph: chrono's default `wasmbind` beside
getrandom keeps both on crates.io's. The alternative was for each of
rust-js's crates to depend on crates.io's under another name, pinned
exactly, and re-export it whole. A types-alone build would then see
upstream's whole API, and a call into what isn't covered would stay
refused one call at a time:

```toml
[dependencies]
upstream = { package = "wasm-bindgen", version = "=0.2.129" }
```

```rust
pub use upstream::*;
```

Tried, with a dependent asking `wasm-bindgen = "0.2"` and `js-sys = "0.3"`,
the app patched to rust-js's 0.2.130 and 0.3.107, cargo 1.99, a fresh
lockfile:

```text
warning: patch `wasm-bindgen v0.2.130 (..)` was not used in the crate graph
      Adding js-sys v0.3.106 (available: v0.3.107)
      Adding wasm-bindgen v0.2.129 (available: v0.2.130)
      Adding wasm-bindgen-macro v0.2.129 (available: v0.2.130)
```

**Cargo unifies each `^0.2` dependent onto the exact-pinned registry
release already in the graph, so rust-js's crates drop out.** The pin
can't be avoided: upstream pinned at rust-js's own release resolves to
the patch, a cycle, and no semver-incompatible release of upstream exists
to hold it apart. And were it allowed, each type rust-js's crates define,
`JsValue`, `Uint8Array`, `Date`, would be another type than upstream's.
A dependent that gives an uncovered upstream item where a covered one goes,
`JsValue::from(js_sys::Date::new_0())`, would fail to type-check in its
types-alone build: #1129 again, crate by crate. (A) stays the one
mechanism, and coverage grows in the order crates share graphs: chrono's
`wasmbind` first, then uuid's `js`.

## Amendment: chrono's `wasmbind` (2026-11-10, #1121)

**chrono 0.4 is covered.** Its `wasmbind`, on by default, uses
`js_sys::Date`: made now, `new_0()`; of milliseconds,
`new(&JsValue::from_f64(ms))`; and of local parts,
`new_with_year_month_day_hr_min_sec`. It reads `get_time()` and
`get_timezone_offset()`. rust-js's `js_sys::Date` is a class, `Date`
(`rust_js::types`), that derefs to `js::Date`, so its methods are
`js::Date`'s own bindings. Only js-sys's names of `new Date(..)` are
bindings of its own. `JsValue::from_f64` is the number itself (`link_name
= "this"`). Through it, chrono's `Utc::now()`, `Local::now()` and `Local`'s
offset of a date print what upstream's JS would: `new Date(year >>> 0,
month0, 15, 12, 0, 0).getTimezoneOffset()`.

Three things the real chrono found, each general:

- **The graph is rust-js's target's.** Cargo's metadata, unfiltered, has
  each platform's dependencies, chrono's `cfg(unix)` iana-time-zone among
  them, which uses js-sys on wasm32 and isn't covered. It held the patch
  off a graph that never builds it. `cargoBuild` reads the metadata with
  `--filter-platform wasm32-unknown-unknown`.
- **A lock of crates.io's holds against the patch.** A crate's own packed
  `Cargo.lock`, or an app's from before, locks js-sys at another release
  than rust-js's, and Cargo keeps it, the patch unused. Where the patch
  starts naming a crate, `cargoBuild` moves each such lock onto it, `cargo
  update -p js-sys`, as Cargo's warning says to. A pin is refused only
  where it's an exact `=` of another release. Seen through the Crater,
  whose copy of chrono packs its lock: git stand-ins can't hold a
  registry's lock, so no test of the suite's does.
- **A dev-dependency only integration tests use isn't built by the
  Crater**: chrono's wasm32 `wasm-bindgen-test`, which `tests/wasm.rs`
  alone uses and which isn't covered, kept the patch off its tests' graph.
  The Crater's JS copy drops a dev-dependency the library's source never
  names (`libraryTestsOnly`). Both its runs have `TZ` set to
  `America/New_York`, a zone with daylight saving time, so a test of
  `Local` is asked of one zone whatever the machine's.

Held by `test/compat.test.ts`: a stand-in of chrono's surface, its
offsets in New York of a winter's and a summer's date, 300 and 240
minutes, native Rust's; and a `cfg(unix)` dependency using js-sys that
doesn't keep the patch off. The real chrono 0.4.45, with its default
features, through `rust-js build`, matched native Rust in `America/New_York`
(-18000 and -14400 seconds) and `Asia/Kolkata` (19800). The Crater's run
of chrono's own tests now gets past wasm-bindgen, to chrono's `tz_data.rs`,
which its tests build: `CStr::from_bytes_with_nul_unchecked` and
`io::Read::read_exact`, gaps of their own (#1153).

## Amendment: wasm-bindgen-test, and a test's features (2026-11-10, #1121)

uuid's own tests, as many crates' are on wasm32, are `#[wasm_bindgen_test]`
under `src/`, so the Crater can't leave its dev-dependency out.

- **rust-js has its own wasm-bindgen-test and wasm-bindgen-test-macro**
  (0.3.80), of the catalog. `#[wasm_bindgen_test]` is a `#[test]`, which
  `rust-js test` runs as JS. An `async` one is a compile error saying
  `rust-js test` runs none yet, and so is one of options, `unsupported =
  test`: never a test left out. `wasm_bindgen_test_configure!(..)` is
  nothing. Where a test runs, `run_in_browser`, is `rust-js test`'s
  runner's, which the crate's configuration names (ADR 1076): its tests
  already run in a JS host, and one that needs a browser says so with
  `#[cfg_attr(not(browser), ignore)]`.
- **The patch's gate counts the crates reached past none of rust-js's**:
  crates.io's wasm-bindgen-test's own wasm-bindgen-futures, which isn't
  covered, is of a subtree the patch replaces, and no longer holds it off.
- **`rust-js test` takes Cargo's `--features` and `--no-default-features`**,
  given as they are, and the Crater gives both runs those its row names,
  its default ones off, as `docs/crate-corpus.md` says a row's are.

Held by `test/compat.test.ts`: a crate's `#[wasm_bindgen_test]`s run by
`rust-js test --features extra`, the one of that feature's too, and an
`async` one refused, saying why.
