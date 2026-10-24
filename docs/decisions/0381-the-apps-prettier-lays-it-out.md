# 0381. The app's Prettier lays out what rust-js writes

Status: Accepted. Amends [0117](0117-output-hooks.md) and
[0207](0207-declarations-printed-by-typescript.md); delivers the layout
rule of [0378](0378-one-file-at-a-time.md). Amended below: the app's
build only, not a file compiled alone; and its config, for where the
file goes, beside its Rust.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): how the JS is laid
out, not what it means.

## Context

An app checks its files' layout, react.dev's `prettier --list-different`
in its CI, and the JS rust-js wrote beside a Rust file is one of them.
rust-js laid it out by oxfmt with `[package.metadata.rust-js.format]`'s
options, a second copy of the app's `.prettierrc`, which the two could
say differently, and which a change to rebuilt nothing. Copied exactly,
oxfmt's layout still isn't Prettier's everywhere. And a crate's `.d.ts`
was TypeScript's printer's, four spaces and double quotes, whatever the
app said: a trial on react.dev failed its check on both.

## Decision

**Where Prettier finds a config for a file rust-js writes, as the app's
`prettier --check` does, the app's Prettier lays it out, by that config:
its JS and its `.d.ts`, all of a build's in one session of the app's
Prettier, run where the crate is (`prettier.rs`). Elsewhere, an app
without Prettier or a file it has no config for, rust-js's own layout
stands, oxfmt's.**

- **Prettier answers its own question**: `resolveConfigFile` and
  `resolveConfig(file, { editorconfig: true })`, so a JSON, YAML or JS
  config, a `package.json`'s `prettier`, its `overrides` and the app's
  `.editorconfig` are read as Prettier reads them, nothing of it copied.
- **Prettier 2's too**, react.dev's: CommonJS, whose API an import finds
  as its module's `default`, where Prettier 3's is its module's.
- **A config, not Prettier installed, decides**: an app relying on
  Prettier's defaults has one, `{}` in `.prettierrc`. rust-js's own
  checkout has Prettier and no config, and its corpus keeps rust-js's
  layout.
- **The JS's map follows**: Prettier's layout of a module is taken as a
  transform's is (ADR 0117), only if it's the same program, the map moved
  to it; else a warning, and rust-js's layout. A node either layout
  moves onto another line is given the mapping it shared on its old one,
  `setCount(` as `() =>`'s, so its line points into the Rust.
- **Cargo is told of the configs Prettier read**, in its record of the
  sources: a change to one builds again. A config made where none was is
  seen by the next build of the Rust.
- **`[package.metadata.rust-js.format]` is gone**, an error that names
  `.prettierrc`; and a transform that runs Prettier is refused, a second
  Prettier after rust-js's own.

## Consequences

- An app whose CI checks Prettier's layout passes on what rust-js wrote,
  as written.
- A build with a config runs Node once more, about 50 ms, and Prettier on
  each file it writes.
- The `.d.ts` of an app without a Prettier config is TypeScript's
  printer's, as before (ADR 0207).

## Amendment: the app's build, not a file compiled alone

An app's Rust is compiled one way, `rust-js build`, through Cargo
([0412](0412-rust-js-build-is-its-own-process.md)); a file compiled alone
is a test's, rustc's suite's or the playground's, and has no crate
([0414](0414-a-library-on-npm-as-rescripts-are.md)). Yet the rule above
ran for a file compiled alone too, from the nearest `Cargo.toml` above it,
maybe another's: each of the corpus's 1,400 compiles a run started Node
and imported Prettier from rust-js's own checkout to be told there's no
config. That was 30 ms of a small case's 80, and a third of the corpus's
compile time.

**The app's Prettier lays out the app's build, Cargo's; a file compiled
alone keeps rust-js's layout**, as its module header names no crate
(0414). Its hooks are still the `Cargo.toml`'s above it (0117): a hook is
named there, where Prettier is found from the file, and a file's JS that
no build wrote is nobody's `prettier --check`.

- `prettier.rs` asks only when `settings.cargo`.
- The tests of the layout build their crate through Cargo, as an app is
  built; "a file compiled alone keeps rust-js's layout, whatever Prettier
  config is above it" holds the rule, and the mutation
  `prettier-for-a-file-compiled-alone` proves it. "A crate with Prettier
  and no config of its own keeps rust-js's layout" is what the corpus
  showed before, now of a Cargo build.
- A small corpus case compiles in 52 ms, not 85.

## Amendment: the config for where the file goes

Prettier was asked for each file's config by the path Cargo's build wrote
it at, in the app's target directory,
`node_modules/.cache/rust-js/target/wasm32-unknown-unknown/debug/deps/rust-js/<hash>/src/ReactLazy.js`.
A config's own options were found from there, as Prettier looks up from a
file to its config; an override by path wasn't. React's `.prettierrc.js`
says `trailingComma: 'es5'`, and `trailingComma: 'all'` for its
`esNextPaths`, `packages/*/src/**/*.js`, which a path in the target
directory never is: every file of the port had es5's commas where React's
own have `all`'s, and its `prettier --check` failed on each.

**Prettier lays out each file by its config for where the file goes,
beside its Rust, as the app's `prettier --check` reads it**: its JS's
path in source, `src/ReactLazy.js` of `src/ReactLazy.rs`, and its
`.d.ts` beside it. That path is the one the manifest says,
`OutputPlan::in_source`'s, the one answer to where a module's JS goes in
source (ADRs 0041, 0273), asked, not made again.

- A crate of an npm package's (ADR 0414) is laid out by the config for
  where its JS is, the package's, as its author's build laid it out.
- A crate of crates.io's (ADR 0415) is run where Cargo's registry has it,
  which has no Prettier: rust-js's layout, as before.
- The test "Prettier lays a file out by its config's overrides for where
  the file goes, beside its Rust" holds it: an override of `src/**/*.{js,ts}`
  lays out the JS and the `.d.ts`; the mutation
  `prettier-at-the-built-path` proves it.
