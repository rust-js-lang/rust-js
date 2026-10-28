# Building the application

You compile an app's Rust with one command, `rust-js build`, run in the
app's directory. It writes each module's JavaScript beside its Rust, and
your dev server or bundler, Vite's or Next.js's, takes it from there.

```bash
rust-js build                       # the package of the Cargo.toml here
rust-js build --package frontend    # a workspace's package: the app's client
rust-js build --check               # writes nothing; fails if a file is stale
```

There's no watch mode and no bundler plugin: run `rust-js build` again
after you change the Rust ([ADR 0412](../../decisions/0412-rust-js-build-is-its-own-process.md)).
Two other commands lint it, `rust-js clippy` ([below](#linting-with-clippy)),
and run its `#[test]`s as JS, `rust-js test` ([Testing](testing.md#rust-tests-compiled-to-javascript)).

## What you need

An app is a Cargo package and an npm package in one directory.

`Cargo.toml` names the app's crate and the crates rust-js ships, by
version. This is what `bun create @rust-js` writes:

```toml
[package]
name = "app"
version = "0.0.0"
edition = "2024"
publish = false

[lib]
path = "src/App.rs"

[dependencies]
js = { package = "rust-js-builtins", version = "~0.0.2" }
react = { package = "rust-js-react", version = "~0.0.4" }
webapi = { package = "rust-js-webapi", version = "~0.0.3" }

# rust-js compiles with `--cfg=rust_js`: tell Cargo it's expected.
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ["cfg(rust_js)"] }
```

- **The crate is a library.** Its root is `src/lib.rs`, or the file
  `[lib] path` names. That file's name is its JS file's name:
  `src/App.rs` is `src/App.jsx`.
- **The `[lints.rust]` table is needed.** The crates' attributes are
  `cfg_attr(rust_js, ..)`, so a plain `cargo check` or your editor reads
  your crate without rust-js's tool ([ADR 0113](../../decisions/0113-plain-rustc.md)).
- **The crates are npm packages.** See
  [Dependencies](dependencies.md) for what each is and how Cargo finds it.

`package.json` has the compiler, the build command, and the packages the
JS imports, all at one version:

```json
{
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "rust-js build --check && vite build"
  },
  "dependencies": {
    "@rust-js/builtins": "0.0.2",
    "@rust-js/react": "0.0.4",
    "@rust-js/runtime": "0.0.4",
    "@rust-js/std": "0.0.4",
    "@rust-js/webapi": "0.0.3"
  },
  "devDependencies": {
    "@rust-js/build": "0.0.4",
    "@rust-js/native": "0.0.4"
  }
}
```

`@rust-js/build` has the `rust-js` command. `@rust-js/native` is the
compiler it runs. `@rust-js/runtime` and `@rust-js/std` are what the
written JS imports at run time (see
[JavaScript environments](environments.md#the-runtime-package)).

And once per machine, the Rust release rust-js is built with:

```bash
rustup toolchain install 1.99.0 --profile minimal --target wasm32-unknown-unknown
```

## Run it beside your dev server

Two processes, each in its own terminal:

```bash
bun run dev          # Vite or next dev, as always
bunx rust-js build   # after each change to the Rust (npx with npm)
```

`rust-js build` runs once and exits. The dev server sees the files it
wrote as any edit, and Fast Refresh keeps the page's state.

A compile error is printed in `rust-js build`'s terminal, which exits with
status 1, and nothing is written: the page keeps running the JS that last
compiled. See [Compile errors](errors.md).

## What it writes

For each module, three files beside its Rust:

```
src/
├─ lib.rs
├─ lib.js          its JavaScript
├─ lib.d.ts        its TypeScript declarations
├─ lib.js.map      its source map, back to lib.rs
├─ shapes.rs
├─ shapes.js
├─ shapes.d.ts
└─ shapes.js.map
```

```rust
// src/lib.rs
mod shapes;

pub use shapes::area;

pub fn total_area(radii: &[f64]) -> f64 {
    radii.iter().copied().map(area).sum()
}
```

```rust
// src/shapes.rs
pub fn area(radius: f64) -> f64 {
    std::f64::consts::PI * radius * radius
}
```

```js
// src/lib.js
import { area } from "./shapes.js";

export { area } from "./shapes.js";

export function totalArea(radii) {
  return radii.map(area).reduce((a, b) => a + b, -0);
}
```

```ts
// src/lib.d.ts
export { area } from "./shapes.js";

export function totalArea(radii: number[]): number;
```

- **One JS file per module.** A module with JSX is `.jsx`, the rest
  `.js`. Where each module's file goes is [Emitting modules](modules.md)'s.
- **A `.d.ts` for every module that has something `pub`.** TypeScript
  that imports the JS gets Rust's types. Under a Next.js app's Pages Router,
  `pages/` and `src/pages/`, no `.d.ts` is written: Next.js would take it
  for a route ([ADR 0276](../../decisions/0276-no-declarations-in-routes.md)).
- **A source map for each JS file.** Your browser's devtools and a test
  runner's stack traces point at the Rust. A frame keeps the JS's name,
  `at beginWork (src/lib.rs:7:9)`: the map gives no other ([ADR 0018](../../decisions/0018-print-with-oxc.md#amendment-the-map-names-nothing)).
- **Each file starts with its header**, `Generated by rust-js from
  src/lib.rs. Do not edit.`, naming its Rust from the crate's directory,
  and a JS file ends with its `//# sourceMappingURL=` line. This guide's
  examples leave both out.
- **Only what changed is written.** A file whose bytes are the same keeps
  its modification time, so the dev server reloads only what changed.

### Layout

Where Prettier finds a config for a file, `.prettierrc` or a
`package.json`'s `prettier` key, the app's Prettier lays out the JS and the
`.d.ts` by it, so `prettier --check` passes on what rust-js wrote. It's
the config for where the file goes, beside its Rust, so an override by
path, `{ files: "packages/*/src/**/*.js", options: { .. } }`, is the
file's. Without one, rust-js lays it out its own way ([ADR 0381](../../decisions/0381-the-apps-prettier-lays-it-out.md)). There's no rust-js setting
for layout. A layout of Prettier's that would change the program isn't
taken: the file is rust-js's layout, and `rust-js build` prints a
`warning: rust-js: the app's Prettier ..` that says why.

Either way, an object is on lines of its own where its struct literal is
written across lines, as rustfmt writes one too long for a line, and on
one line where it's written on one and fits, as Prettier keeps an object
a person wrote ([ADR 0835](../../decisions/0835-an-object-is-across-lines-where-its-literal-is.md)).

## Commit the JS, not the maps

Commit each `.js`, `.jsx` and `.d.ts` rust-js writes. A diff shows what a
change to the Rust did to the output, and a teammate who doesn't write Rust
builds the app from what's committed, with no compiler ([ADR 0041](../../decisions/0041-react.md)).

Don't commit the source maps, Cargo's files, or the patch `rust-js build`
writes. In `.gitignore`:

```gitignore
# The JS rust-js writes beside the Rust is committed; its maps aren't.
*.js.map
*.jsx.map

# What Cargo writes, and the patch `rust-js build` writes of what's installed.
/target
/Cargo.lock
/.cargo
```

## Check what's committed in CI

`rust-js build --check` compiles and compares, and writes nothing. It
fails, naming each file, where what's committed isn't what the Rust
compiles to:

```
rust-js: the JS beside the Rust isn't what it compiles to; run `rust-js build`:
  src/shapes.js
```

Run it before your bundler's build, as the `build` script above does. Source
maps aren't compared, nor an npm package's JS, which isn't yours to commit.

## A Cargo workspace

In a workspace whose root `Cargo.toml` is only `[workspace]`, name the
app's package:

```bash
rust-js build --package frontend
```

Without it, `rust-js build` stops:

```
rust-js: /path/to/app/Cargo.toml is a workspace's: name the app's package, `rust-js build --package <name>`
```

Every crate of the workspace that `frontend` uses gets its JS beside its
Rust too, and `frontend` imports it there, a function or a `const` alike
([ADR 0770](../../decisions/0770-a-librarys-const-is-imported-by-its-name.md)):

```
models/src/lib.rs     pub fn valid_email(email: &str) -> bool
models/src/lib.rs     pub const MAX_LENGTH: usize = 254;
models/src/lib.js     export function validEmail(email) { .. }
models/src/lib.js     export const MAX_LENGTH = 254;
frontend/src/lib.js   import { MAX_LENGTH, validEmail } from "../../models/src/lib.js";
```

The package built is compiled as a program only where no crate of the
workspace uses it; one another crate uses is a library, whichever package
is built ([ADR 0360](../../decisions/0360-the-app-is-a-program.md)). A
library's generic function takes the copies and drops its own crate's
calls give it, as a program's does, so a crate's JS is the same whether
another crate uses it or not, the one its JS callers call:
`createContext(defaultValue)`, `useState(initialState)`. A crate whose call
gives one a value to copy or drop that it takes nothing for, a struct it
changes in place, a value with a destructor, compiles its own of it from
the library's MIR, beside its first caller, and calls that; the library's
private functions it calls too. One that reads its library's private
`static` or thread-local, or calls a binding its library names by a path
relative to its own file, is refused, naming it
([ADR 0914](../../decisions/0914-a-librarys-generic-function-is-its-programs.md)):

```
models/src/lib.rs     pub fn pick<A, B>(a: A, _b: B) -> A
models/src/lib.js     export function pick(a, b) { return a; }
frontend/src/lib.js   function pick(a, b, dropB) { dropB?.(b); return a; }
```

So are its trait impls and copies: a library writes and exports what its
program does. A crate that uses an impl its library's JS hasn't, a `Hash`
it gives generic code, a derived `Debug` it prints, a destructor, compiles
its own of what it uses of it; one whose dictionary the library exports,
it calls through that, `strToKebabCase.toKebabCase(title)`. A library
copies what its own crate changes in place; a crate that changes a value
a library's function copies compiles its own of that function, which
copies it
([ADR 0914, amended](../../decisions/0914-a-librarys-generic-function-is-its-programs.md)):

```
models/src/lib.rs     pub fn first(points: &[Point]) -> Point { points[0] }
models/src/lib.js     export function first(points) { return $index(points, 0); }
frontend/src/lib.js   function first(points) { return { ...$index(points, 0) }; }
```

The same `models` crate builds natively for a Rust server. See
[Testing](testing.md#test-shared-logic-natively).

## Run your tools on what it writes

A crate's `Cargo.toml` can name checks that run on the JS once a build has
written it, a type checker or a linter:

```toml
[package.metadata.rust-js.hooks]
check = [
  { run = ["tsc", "--noEmit", "--pretty", "false", "-p", "."] },
]
```

- **`{files}`** in `run` is every file the build wrote, one argument each.
  The command runs in the crate's directory.
- **`fail`** is `"build"`, the default, where a failure fails the build, or
  `"never"`, where what it says is printed as warnings.
- **What it says of the JS is said of the Rust.** A line that names a
  written file at a line and column, `file:line:col:` or `file(line,col):`,
  is moved through the source map to the Rust, and named for its tool:

  ```
  src/shapes.rs:1:8: [tsc] error TS7006: Parameter 'radius' implicitly has an 'any' type.
  ```

`transform` hooks, commands each module's text goes through before it's
written, are there too ([ADR 0117](../../decisions/0117-output-hooks.md)); a transform that runs Prettier is
refused, as rust-js runs the app's Prettier itself.

## Incremental builds

Cargo decides what's rebuilt. A crate whose Rust, dependencies and features
haven't changed is fresh, and its JS is the JS that build wrote.

- **Cargo builds in `node_modules/.cache/rust-js/target`**, out of what your
  bundler watches. Set `CARGO_TARGET_DIR` to build elsewhere.
- **A file rust-js wrote that a build no longer writes is removed**: a
  module you deleted, a crate you stopped using. Only if it's still as
  rust-js wrote it: a file you edited is yours, and stays.
- **What a build wrote is recorded in Cargo's target directory**,
  `rust-js-in-source.json`. `cargo clean` forgets it, and the next build
  removes nothing it didn't write itself.
- **What rust-js wrote in Cargo's target directory isn't yours to change.**
  A crate's JS there that's gone or edited, of a build Cargo has as fresh,
  is refused, with the command that builds it again:

  ```
  <target>/…/app-9cb45400a0e35ae6/lib.js isn't what rust-js wrote for Cargo's build of app, which has it as done: `cargo clean -p app --target wasm32-unknown-unknown` to build it again
  ```

## Which compiler it runs

`rust-js build` runs, in order:

1. the compiler `RUST_JS_COMPILER` names, if it's set;
2. the app's `@rust-js/native`;
3. a rust-js checkout's own `target/debug/rust-js`, when `@rust-js/build`
   is that checkout's, built from the checkout's sources first
   ([ADR 0854](../../decisions/0854-a-checkouts-compiler-is-built-before-it-runs.md)).

The first two are run as they are. The checkout's is built each time by
its `bun run build:compiler`, Cargo's build, which does nothing when its
sources haven't changed and rebuilds when they have, a `git pull`'s say:
a checkout never runs a compiler older than its sources. What Cargo says
comes first. A build that fails stops, and the compiler an earlier build
left isn't run:

```
rust-js: building /path/to/rust-js/target/debug/rust-js from this checkout's sources failed, as it says above; the one an earlier build left isn't run: fix it, or run `bun run build:compiler` in /path/to/rust-js
```

Without one, it stops, and your bundler still builds what's committed:

```
rust-js: no rust-js at /path/to/rust-js: install rust-js or build it with cargo build to compile the Rust
```

The compiler and `@rust-js/runtime` and `@rust-js/std` must be one
version:

```
the JS rust-js 0.0.4 writes imports @rust-js/runtime 0.0.4: install it
```

## Your editor

rust-analyzer checks the app through rust-js, which reads inside `jsx!`
as a plain rustc doesn't. A created app has it in `.vscode/settings.json`
([ADR 0222](../../decisions/0222-editor-check.md)):

```json
{
  "rust-analyzer.check.overrideCommand": ["./node_modules/.bin/rust-js-check"]
}
```

`rust-js-check` builds in its own target directory, so it neither waits
for `rust-js build` nor makes it build again. rustc's warnings show there,
of what JS uses too: a struct's fields a binding is given, `&Logs { .. }`
as a `&dyn PropertyDescriptorMap`, and a function only
`js::export_default!` names are used, as JS reads them. A plain `cargo
check` or `cargo clippy` runs no rust-js and warns of them as dead code
([ADR 0440](../../decisions/0440-a-binding-needs-no-allow.md)): lint
with `rust-js clippy`.

## Linting with clippy

`rust-js clippy` is the app's `cargo clippy`: clippy's lints, as rust-js
reads the Rust, inside `jsx!` too, with what JS uses used. Its lint flags
come after `--`, as `cargo clippy`'s do, and `clippy.toml`,
`[lints.clippy]` and `#[allow(clippy::..)]` are clippy's own
([ADR 0888](../../decisions/0888-rust-js-clippy.md)):

```bash
rust-js clippy                       # the package of the Cargo.toml here
rust-js clippy --package frontend    # a workspace's package
rust-js clippy -- -D warnings        # as CI runs it: a finding fails it
```

It writes no JS, in its own target directory, so it neither waits for
`rust-js build` nor makes it build again. Only the app's own crates are
linted, as `cargo clippy` lints no crate of crates.io's. A variable only
JSX reads, a struct a binding is given, `&Logs { .. }`, and a function
only `js::export_default!` names are used, so what it says is what
clippy finds of the Rust:

```
error: this `if` statement can be collapsed
  --> src/App.rs:18:5
```

The playground has no clippy: it's the native compiler's.
