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
