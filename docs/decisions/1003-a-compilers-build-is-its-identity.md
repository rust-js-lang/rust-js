# 1003. A compiler's build is part of its identity

Status: Accepted. Extends the compiler identity of manifests (ADR 0100)
and of `rustc -vV` through rust-js, which Cargo hashes into each crate's
fingerprint.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; what one compiler
decided is never read by another build of it.

## Context

React's reconciler landing agent saw ADR 0852's refusal, "this crate
counts an `Rc<ReactContext<..>>` .. but `react_isomorphic` .. counts none",
after rust-js's main checkout moved on: the same sources built cleanly in
a fresh copy, and touching `shared`'s and `react`'s roots, which rebuilt
them, made it go away. Their libraries' manifests had been written by
another build of the compiler.

A compiler's identity was its version, its toolchain and its manifests'
ABI: `rust-js: 0.0.4, Rust 1.99.0, ABI 1`. Every build of one version
said the same. So a library's manifest, whose facts are what that build
decided (which `Rc`s it counts, whether its `dyn Fn`s drop, which drops
and copies its exports take), was read as this build's by any other build
of the version. And Cargo, whose fingerprint of a crate hashes what `rustc
-vV` says, had a crate another checkout's or worktree's compiler built as
done: the compiler a crate's dep-info names is that other one, whose file
hadn't changed. The library's contract version (`library::VERSION`, 10)
is bumped by hand where its fields change shape, not where what fills them
does.

## Decision

**A compiler's identity includes its build: a hash of what it's made of,
its sources (with the runtime helpers and the std manifest they build in),
its toolchain file, its build script, and its package's manifest and lock
file** (`build.rs`'s `RUST_JS_BUILD`, `manifest::BUILD`).

- **Manifests say it**, `"compiler": { .., "build": "52c100506ffb5402" }`,
  and a consumer reads a library's only where it's its own build's, as
  before: "incompatible dependency compiler identity". A manifest of a
  compiler before this one has none, and is refused the same way.
- **Cargo is told it**: `rust-js: 0.0.4, Rust 1.99.0, ABI 1, build
  52c100506ffb5402`, so a crate another build compiled is built again.
- **`@rust-js/std`'s manifest says none** (`Manifest::of_no_build`): it's
  committed, built into each build of the compiler, which doesn't check
  its identity.
- **The browser's compiler**, `wasm/`, builds with the same script, its
  own lock file among what it hashes; its rpath, the toolchain's, is the
  native compiler's alone.

## Why

- **One source of a fact, its version checked**: what a library's
  manifest says is a cache of what its compiler decided, and a cache is
  keyed by what made it (AGENTS.md). A build that decides otherwise reads
  none of another's.
- **The hash of the inputs, not of the binary**: the same sources give the
  same identity on every machine, and asking it costs nothing at run time;
  hashing a 100 MB compiler at each crate's compile would.

## Alternatives

- **Bump `library::VERSION` with each change to a fact**: by hand, and
  forgotten where a field's meaning, not its shape, changes, as here.
- **The binary's modification time**: Cargo already reads it, of the one
  compiler a crate's dep-info names; another checkout's compiler is another
  file, as old as it is.

## Consequences

- Each change to the compiler's sources rebuilds every crate of a Cargo
  build that compiler runs, as a change to the binary already did of the
  same checkout's.
- Libraries built by a compiler before this one are refused until rebuilt.

## What holds it

- `test/cargo-workspace.test.ts`, "Cargo is told which rust-js it runs":
  the build in `rustc -vV`'s line, as `--version-json` says it, and in
  Cargo's `.rustc_info.json`.
- `test/cargo-link.test.ts`, "a real Cargo dependency links": a library's
  manifest of another build, or of none, is refused.
- `test/manifest.test.ts`: the host reads a build of sixteen hex digits.
- `test/std-package.test.ts`: std's manifest says no build.
- Mutations `cargo-version-without-build` (`cargo.rs`),
  `manifest-build-unread` (`manifest.rs`) and `std-manifest-of-a-build`
  (`output.rs`).

## Amendment: the `@rust-js/typescript` that prints its `.d.ts` (#675)

Case ([0262](0262-when-rust-and-js-disagree.md)): none, as above.

A regen of React's port kept what a compiler before it had written, until
`node_modules/.cache/rust-js` was cleared by hand: reported as the cache
keying the compiler by its path. It doesn't: what Cargo is
told, `rustc -vV`'s line, says the build, and a crate's dep-info names the
binary, so a compiler rebuilt at the same path rebuilds every crate, and
one swapped in older than the cache does too, by its build. But what
rust-js writes isn't all the binary's. A crate's `.d.ts` is printed by
`@rust-js/typescript` (ADR 0207), `rust-js build`'s own, which it names in
`RUST_JS_TYPESCRIPT` (ADR 0412): a checkout's `typescript/index.js`, read
at run time. It was in neither the build's hash nor the dep-info, so a
commit that changed it alone, 2301d08f4's quoting of a member's name, left
every crate fresh, and its `.d.ts` the printer before it had printed.

**Cargo is told the printer too**: `rust-js: 0.0.4, Rust 1.99.0, ABI 1,
build 52c100506ffb5402, declarations 4081613b597496de`, a hash of the
module `RUST_JS_TYPESCRIPT` names and of its package's manifest, which pins
the TypeScript it runs exactly (`typescript::printer`). Another printer at
the same path, of another commit or copied in as old as the cache, is
another build.

- **Of what it is, not its date**: the dep-info's dates miss a file copied
  in older than the cache, as an installed compiler's are (above).
- **Asked each build**: Cargo keeps what `rustc -vV` says by the size and
  dates of its rustc and its wrapper, neither of them the printer. The
  tooling writes rustc's shim anew for each build (`rustcShim`), so Cargo
  asks again each time; a build that kept the shim would keep the line.
- **Not in a manifest's identity**: a library's facts, which a consumer
  reads, aren't the printer's, and `--version-json` stays the binary's.
- **Not the app's Prettier**, which lays the JS out (ADR 0381): the app's
  input, not the compiler's, its configs in the dep-info. Which Prettier
  is installed is in the line too, since #684 (below).

Each change to the printer rebuilds every crate of a Cargo build, as a
change to the compiler's sources does, its files kept beside the others'
(a toolchain release's `-C metadata` hashes the line too).

What holds it: `test/cargo-workspace.test.ts`, "Cargo is told which
declarations printer rust-js runs": a crate's `.d.ts` built, the printer
changed at the same path and dated as before, and the next build's `.d.ts`
the new printer's. Mutations `cargo-version-without-printer` (`cargo.rs`),
`printer-unnamed` and `printer-of-its-package-alone` (`typescript.rs`).

## Amendment: the app's Prettier (#684)

Case ([0262](0262-when-rust-and-js-disagree.md)): none, as above.

The app's Prettier lays out the JS rust-js writes (ADR 0381), each file by
the config Prettier finds for it, which a crate's dep-info names. Which
Prettier was in no key: an upgrade of the app's, `yarn upgrade prettier`,
lays the JS out otherwise, but it's a package of `node_modules`, no file
of which Cargo dates, so every crate stayed fresh with its old layout.

**Cargo is told it too**: `.., declarations 4081613b597496de, prettier
<hash>`, a hash of the manifest of the Prettier each crate rust-js compiles
finds, the app's own and crates.io's (`RUST_JS_CRATES`,
`RUST_JS_DEPENDENCIES`), with its version, by each crate's directory
(`prettier::identity`). It's found as the layout's session imports it, from
where the crate is: Node's `findPackageJSON("prettier", <crate>/lib.js)`,
all of them in one session.

- **By its manifest**: an installed package's files are its release's,
  which its version says; hashing all of Prettier's would read megabytes
  each build for nothing more.
- **Of every crate at once**: the line is one for the build. A crate
  added, or a Prettier installed for one of them, rebuilds every crate, as
  a change to the compiler does; that's as rare as it's cheap to be right.
- **None where no crate has one**: the line is as before, and a crate's
  JS is rust-js's own layout.
- **A Node session each build**, about 40 ms, beside the layout's own:
  Cargo asks `rustc -vV` each build (above), and an editor's check too.
- **Its plugins too**, since #692 (below).

What holds it: `test/cargo-workspace.test.ts`, "Cargo is told which
Prettier lays out the app's JS": the app's Prettier upgraded at the same
path, each of its files dated as before, and the next build's JS its
layout. Mutations `cargo-version-without-prettier` and
`cargo-prettier-of-no-crate` (`cargo.rs`),
`prettier-identity-of-its-directories-alone` and
`prettier-identity-found-nowhere` (`prettier.rs`).

## Amendment: the plugins of the app's Prettier (#692)

Case ([0262](0262-when-rust-and-js-disagree.md)): none, as above.

A config's `plugins`, react-in-rust's `prettier-plugin-hermes-parser`,
lay the JS out as much as Prettier does, and were in no key: an upgrade
of one alone, Prettier and the config the same, kept every crate's old
layout.

**The `prettier <hash>` of the line hashes them too**: of each crate,
after its Prettier's manifest, each plugin its configs name, by its name
and its package's manifest (`prettier::identity`). Which, and where from,
is asked as the layout would:

- **Each module's config, by Prettier**: `resolveConfig` of the JS beside
  each of the crate's `.rs` files, `.js` and `.jsx` alike, as rust-js
  writes one of them, so an `overrides` entry's plugins are its own. A
  module placed outside its crate's directory by `#[path]` is asked of
  where its Rust is in the crate, which is no config of it: the crate's
  other files' plugins are, but one that only an `overrides` entry for
  that other place names isn't.
- **Each one where Prettier 3 loads it from**, its `loadPlugin`, run where
  the crate is: a `file:` URL, an absolute path, a file of that name
  there, else the package Node finds from there; its manifest the
  `package.json` above it, `findPackageJSON`. Prettier has no API that
  says it, so the session mirrors Prettier 3's rule, which a Prettier
  release that changes it would need followed here.
- **A plugin given as an object**, not a name, is the config's own,
  which the dep-info already names.

Each module's two configs are asked in the one Node session of a build,
Prettier caching each config file it reads: React's port's 17 packages,
their `prettier-plugin-hermes-parser` found, take 0.4 s on the M3 Max
(2026-11-04), where Prettier alone took about 40 ms.

What holds it: `test/cargo-workspace.test.ts`, "Cargo is told which
Prettier plugins lay out the app's JS": a plugin the config names
upgraded alone at the same path, each of its files dated as before, and
the next build's JS its layout. Mutations `prettier-identity-without-plugins`,
`prettier-plugin-found-nowhere` and `prettier-plugins-of-no-file`
(`prettier.rs`).
