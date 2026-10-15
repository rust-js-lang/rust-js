# 0414. A library is shared on npm, as ReScript's are

Status: Accepted. Amends [0101](0101-cargo-workspace-wrapper.md), the
crates rust-js compiles, and [0118](0118-bindings-on-npm-only.md), npm for
a library with code as for a binding, and [0119](0119-community-bindings.md),
a binding crate saying it's one; delivers [0100](0100-separate-crates.md)'s
separate compilation for a crate from npm.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): how a crate is shared
and built, not what a program means.

## Context

npm is rust-js's one channel for what's shared: its compiler, its runtime,
and every binding (ADR 0118). A binding has no JS of its own. A library
with code, a component library or the models a native server shares, had
no way there: Cargo's workspace wrapper (ADR 0101) runs only for the
workspace's members, and a crate an npm package has, which Cargo finds by
the patch (ADR 0118), isn't one, so rustc checked it and rust-js refused a
call into it. Had it been compiled, its JS would have been written beside
its Rust, in `node_modules`, and the app's JS would have imported it by a
relative path, through pnpm's store, `../node_modules/.pnpm/@acme+greet@1.0.0/..`,
which names the version and changes with each install.

ReScript has shared its libraries on npm for years, and its build, rewatch,
behind `rescript build` since v12, answers each part:

- **A library's package is its source, and its compiled JS** for JS users:
  "check in your compiled JavaScript output", so "JS users might not even
  realize that they've installed a library written in ReScript".
- **An app compiles every dependency from source, by its own compiler and
  settings, into the dependency's own directory** in `node_modules`
  (`rewatch/src/build/packages.rs`, `compile.rs`): the root's
  `package-specs` and suffix are every package's. No compiler version is
  shared between a library and an app: each package's `compiler-info.json`
  rebuilds it when the app's compiler changes.
- **An app imports a dependency's module by its package's name**,
  `"@testrepo/dep01/src/Dep01.mjs"` (`js_name_of_module_id.ml`), and its
  own by a relative path; the name is `package.json`'s, or "the path
  resolution will be wrong".
- **A dependency is quiet**: its warnings and dev-dependencies are left
  out, and it isn't watched.

It writes a dependency's JS in place, which a store that links one file
into many projects, pnpm's or bun's, would see changed in each.

## Decision

**A rust-js library is one npm package: its crate, and the JS and
declarations its author's `rust-js build` wrote beside its Rust. A JS app
imports that JS. A rust-js app compiles the crate again, by its own
rust-js, into the package, as ReScript's apps do.**

```
@acme/greet/                       an app's node_modules/@acme/greet, npm's or pnpm's
  package.json   { "name": "@acme/greet", "main": "src/lib.js",
                   "rust-js": { "crate": "acme-greet" } }
  Cargo.toml     [package] name = "acme-greet", version = "1.0.0"
  src/lib.rs     the Rust
  src/lib.js     its JS, and src/lib.d.ts, as rust-js build writes them
```

```js
// a JS app: the JS as its author built it
import { Greeting, greet } from "@acme/greet";

// a rust-js app's src/lib.js, of `greet::greet("Ada").text()`
import { Greeting, greet } from "@acme/greet/src/lib.js";
```

- **rust-js compiles the app's own crates, Cargo's packages of a path**: a
  member of its workspace, a path dependency, and a crate an npm package
  has, which the patch names. A registry's or git's crate is rustc's, as it
  was (ADR 0101). rust-js is Cargo's `RUSTC_WRAPPER`, run for every crate,
  and the tooling says which are its own, `RUST_JS_CRATES`, their
  directories, from `cargo metadata`: Cargo, which resolves them, answers.
- **A binding crate says it's one**, in its own `Cargo.toml`, as rust-js's
  `builtins`, `webapi`, `react`, `next` and `node` do:

  ```toml
  [package.metadata.rust-js]
  bindings = true
  ```

  What it binds is JS's own, so it has no JS to compile: rustc checks it,
  and rust-js reads its metadata (ADR 0024). `cargo metadata` gives the
  tooling what it says, as it gives the rest, where the compiler had a list
  of three names, which `next` and `node`, path dependencies Cargo's
  workspace wrapper never ran for, weren't in.
- **A crate from npm is compiled into its package**, its JS beside its
  Rust, as any crate's is (ADR 0041), by the app's rust-js: a library is
  never held to the rust-js its author had. What the author's build wrote
  there, the app's writes again, and is the same where the two rust-js are.
- **A crate's JS is the same whichever build writes it**: its header names
  its Rust from its crate's directory, Cargo's, `src/lib.rs`, not from the
  app's. A file compiled alone, a test's or the playground's, has no crate
  Cargo says, only the nearest `Cargo.toml`, maybe another's: as it's given.
- **The app imports a package's JS by the package's name**, the file's path
  in it after, `@acme/greet/src/lib.js`, wherever its package manager put
  it; a crate's own modules, and the app's, by a relative path. A package
  installed under another name than its own, an alias's, `"x":
  "npm:@acme/greet"`, is refused: Node finds it only by the name it's
  installed under.
- **A package's files are replaced, never written through**: a store links
  one file into many projects, and each keeps its own.
- **A package isn't the app's**: `rust-js build --check` compares the app's
  JS, not a package's, which isn't committed, and a package's checks (ADR
  0117) aren't run. **Its crate is quiet**: a failed build says the app's
  errors, not what a package's crate warns of, its author's to act on.
- **A package's `exports`, if it has them, export what the app imports**,
  `"./src/*": "./src/*"`: one that hides a file the app's JS imports is an
  error that says so, and what to export, before anything's written.
- **One answer of which crates rust-js compiles**: ADR 0085's planner of
  Cargo's graph, `planCargoLibraries`, which no build used, is removed.

## Why

- **One build, ReScript's, proven**: every crate is compiled by the app's
  compiler, as ADR 0100 has it, so there's no compiled form to keep between
  rust-js versions, and the library's JS is the app's rust-js's, its fixes
  too.
- **A library's JS is readable JS on npm**: a JS app uses it as it uses any
  package, which is the north star.
- **The import names the package, not where it is**: the app's committed JS
  is the same with npm, bun or pnpm, and across installs.
- **It's tested** (`test/libraries.test.ts`): a library its author built,
  installed npm's way and pnpm's, each file linked from a store; the app
  built, its JS importing `@acme/greet/src/lib.js` and running; the
  package's JS the app's rust-js's, the store's unchanged; `--check` of the
  app passing with the package's JS another; and a JS app importing the
  author's JS; a package whose `exports` hide its JS refused, and one
  exporting it imported; and a failed build without the package's
  warnings. `test/patch.test.ts` refuses an alias, and the architecture
  test holds the one answer of which crates rust-js compiles: no list of
  names in the compiler, no workspace wrapper, and each binding crate of
  rust-js's saying it's one.

## Consequences

- **Each module's header names its Rust from its crate's directory**: a
  member's in a workspace changes, `frontend/src/api.rs` to `src/api.rs`,
  as the pilot's did.
- **A community binding says `bindings = true` too** (ADR 0119): one that
  doesn't is compiled by rust-js, as a library is.
- **A JS app needs `@rust-js/runtime`** where the library's JS imports it,
  a dependency of the library's, as `@rescript/runtime` is of ReScript's.
