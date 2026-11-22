# 1181. The compiler's crates

Status: Accepted (2026-11-22, Tu's decision to split the compiler into
crates, #2294; its step 2, #2297, the lead's design). Its first move, M1,
lays the workspace and moves `collections`, `lines` and `decisions`; the
rest of owned output and printing follow, one move each.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This is how the
compiler's code is laid out, not anything the JS does: every move writes
the same JS, the corpus's, std's and the React port's.

## Context

rust-js was one crate, some 287,000 lines. The layers
[the architecture](../architecture.md) draws, driver, front end, owned
output and printing, were held only by `test/architecture.test.ts`'s rules
of which module may name which. Nothing but rustc's front end needs
rustc, yet every part of the compiler built with it, and no part could be
built or tested without it. AGENTS.md asks that contracts be enforced with
types and visibility; a module boundary inside one crate enforces neither,
as `pub(crate)` is the whole crate's.

## Decision

**The compiler is a Cargo workspace.** The binary, `rust-js`, stays at the
checkout's root, `src/`, with the driver and the front end. Each other
crate is a directory of `crates/`, named for its package:

- `rust-js-program`: owned output, the program rust-js writes as data of
  its own (`js`, `program`, `comments`, `names`, `collections`, `lines`,
  `reachability`, `link`, `prepare`, `runtime` with `crates/rust-js-program/src/runtime/*.js`,
  `manifest`, `packages`, `paths`, `settings`, `decisions`, `library`,
  `css_modules`), depending on none of the compiler's crates;
- `rust-js-emit`: printing (`to_oxc`, `format`, `typescript`, `prettier`,
  `node_session`, `output` with `local_js`, `publish`, `hooks`), depending
  on `rust-js-program` alone.

The front end depends on `rust-js-program` only; the driver on both.
#2299 cuts `src/lower/` into crates of its own by the same rules.

**One list.** The root `Cargo.toml`'s `[workspace] members` lists the
crates, and `default-members` all of them, so a plain `cargo build`, `test`
or `clippy` takes each, as check:fast and CI run them. Each other Cargo
package of the checkout, a binding crate, an example, a vendored crate,
the playground's build, is excluded and its own; versions more than one
crate depends on are `[workspace.dependencies]`. `scripts/compiler-crates.ts`
reads the list and each crate's root, and is the one answer to which crate
a file is of (`crateOf`), whether it's the compiler's source
(`isCompilerSource`), its sources' files and directories, its module path,
and which crate a top-level module is of (`crateOfModule`): the
architecture's tests, the owner table, the file-size cap, push and
check:fast, the test map's readers, CI's path filters and the mutations'
workspace all ask it, and none lists `src/` and `crates/` of its own.
build.rs's inputs, the build's identity (ADR 1003), and wasm/prebuilt.sh's
take `crates/` whole.

**Visibility is the contract.** A crate's `pub` items are what its
dependents use, and no more: a move widens exactly what rustc says another
crate names that its own keeps private (`move-paths.ts --widen`), never
every `pub(crate)`. A crate's `pub` items are its API; what it keeps to
itself stays `pub(crate)` or narrower.

**Paths are rewritten, not re-exported.** A moved module is named by its
crate's path, `rust_js_program::collections::IndexMap`, never through a
`use` of the binary's root that keeps `crate::collections` resolving: two
ways to name one module is what AGENTS.md's "one way" forbids.
`bun scripts/move-paths.ts <from> <to> ..`, m3-max's script for moving
files (#2299), rewrites the Rust paths too: which crate a module is of is
the layout's answer, so it takes no map. It rewrites `crate::x`, the
`super`s that reach a root, a root's bare `x::`, and splits a `use` tree's
group whose parts moved to different crates; a path from a crate to a
module of the binary, which no crate can depend on, it says and leaves.
Run with no pairs on a branch after a move lands, it brings the branch's
own code along.

**The layers are the crate graph.** Where a crate holds a layer, Cargo
refuses what the layer rules forbade: owned output can't name the front
end, as it doesn't depend on the binary. `test/architecture-crates.test.ts`
holds each crate's dependencies on the others to the table above, that no
crate but the binary uses rustc (no `rustc_*`, `#![feature]` or `extern
crate`; `.cargo/config.toml`'s `RUSTC_BOOTSTRAP` names the binary alone),
and that every Cargo package of the checkout is a member, excluded or its
own workspace. The module rules of `test/architecture.test.ts` stay for
what's still in one crate, and read another crate's path by its name.

**WASI at every move.** The playground's compiler, `wasm/Cargo.toml`,
depends on the crates by path, as the binary does. check:fast and CI's
lint job build every crate but the binary for `wasm32-wasip1` (`cargo
check --target wasm32-wasip1 --workspace --exclude rust-js`), seconds;
the full WASI build, minutes of rustc's front end, stays CI's `wasm` job.

## Consequences

- A crate other than the binary builds and tests without rustc, on any
  target.
- The test map counts every crate's functions: its counting build runs
  rustc through `scripts/counting-rustc.sh`, its `RUSTC_WRAPPER`, which
  instruments a compile whose `--crate-name` is one of the compiler's
  crates' (`RUST_JS_COUNTED_CRATES`, of compiler-crates.ts) and hands any
  other, a dependency's or a build script's, to the machine's cache; a
  counted crate's build is never cached, as a cache keyed on what it's told
  would hand back one that counts nothing. Not Cargo's workspace wrapper,
  which ADR 0414 keeps out of the checkout. The selector reads a crate's file by the map
  where the map counted one of its functions (`select.ts`'s `counted`),
  and selects everything of one it counted none of, a map made before
  (amended after M1).
- A mutation's unit tests are the binary's (`--bin rust-js`); another
  crate's run by a mutation is M2's, when `js`'s unit-tested mutations move.
- Each move is pushed alone, byte-identical in the corpus, std and the
  React port, with its file list announced first, so the others rebase
  onto it and run `bun scripts/move-paths.ts`.

## Amendment: moves closed under their dependencies (M2, 2026-11-22)

ioEJ's grouping, M2 `js`, `program`, `comments`, `names`, didn't build:
a move must take every module its modules name. Measured of each module's
`crate::` paths and its root's `super::` paths:

| Module | Names |
| --- | --- |
| `js` | `names`, `runtime` |
| `names` | `runtime` |
| `runtime`, `paths`, `reachability` | none |
| `program` | `js`, `library`, `runtime` |
| `library` | `manifest`, `packages`, `paths`, `runtime` |
| `manifest` | `library`, `paths` |
| `packages` | `paths`, `program` |
| `settings` | `paths` |
| `comments`, `css_modules` | `program` |
| `link` | `js`, `program`, `runtime` |
| `prepare` | `js`, `names`, `runtime` |

So the moves are M2 `runtime` (and `runtime/*.js`), `paths`, `names`,
`js`; M3 `program`, `comments`, `library`, `manifest`, `packages`,
`settings`, `css_modules`, `library` and `manifest` naming each other
inside the one crate; M4 `reachability`, `link`, `prepare`; M5 printing,
`rust-js-emit`. The lead approved it.

M2 also settles what a crate move needs beyond paths:

- **A test's conveniences across crates.** `From<&str> for Name` and
  `Pattern`, a test's and never a compile's (#2034), were `#[cfg(test)]`,
  which is the crate's own tests' alone; the binary's tests make them too.
  They're `rust-js-program`'s `testing` feature, which the binary enables
  as a dev-dependency; the workspace's resolver, 3, keeps a dev-dependency's
  features out of a normal build, and `test/architecture-crates.test.ts`
  holds that a normal build's feature tree has no `testing`.
- **Unit-tested mutations of another crate.** A mutation's `unitTests`
  filter runs its file's crate's tests, `cargo test --no-run -p
  rust-js-program --lib` (`unitTarget`), and a build of many's unit tests
  are one crate's.
- **The mutations' own text.** `move-paths.ts` rewrites the Rust of each
  mutation list's `find` and `replace`, so a mutation of a binary file that
  names `crate::runtime::` still applies after the move.
- **Paths written whole.** A path is rewritten only where it isn't the
  end of a longer one, so a second run leaves `crates/rust-js-program/src/
  js.rs` alone.
- **A runtime helper's JS** stays where its module is: the binary's unit
  tests that read `lent.js` and `std_iterator.js` include them from the
  crate's directory.

M3 moves `program`, `comments`, `library`, `manifest`, `packages`,
`settings` and `css_modules`, and with them their crates.io dependencies,
serde, serde_json, lightningcss (`css_modules.rs` its one reader, so the
binary and the playground's build no longer name it) and toml (natively
only), versions more than one crate takes being the workspace's:

- **The build's identity** stays `build.rs`'s (ADR 1003), the binary's: a
  crate built apart has no `RUST_JS_BUILD`, so the binary gives it to
  `manifest::set_build` as it starts, and `manifest::build()` reads it,
  refusing to answer before; no second build script hashes the checkout.
- **A test-support module** another crate's tests read, `library::scopes`
  (the contract's facts, ADR 1129), is the `testing` feature's, its items
  `pub`.
- **`--widen` builds every target**, so what a crate's tests name of
  another's is made `pub` too.
- **A file it includes from the checkout**, `std/manifest.json`,
  `docs/library-contract.md`, `rust-toolchain.toml`, is included by its
  path from the crate's own directory.
- **rustfmt lays a lengthened path out anew**, which can move a mutation's
  `find` off its line: after a move, `bun scripts/format.ts`, then test
  that each mutation applies (`test/mutations.test.ts -t applies`), and
  repoint those that don't, as M2's push found eight.

M4 moves `reachability`, `link` and `prepare`, with no dependency of
their own: owned output's crate then holds all of it but `output`,
`publish` and `hooks`, which call printing and go with it at M5.
