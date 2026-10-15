# Build an app: `rust-js build`

An app's Rust is compiled one way: `rust-js build`, `@rust-js/build`'s
command, run in the app's directory, its own process beside the app's dev
server or build, `next dev` or `vite`, which are the host's alone (ADR 0412).

```sh
rust-js build                       # the package the app's Cargo.toml is of
rust-js build --package frontend    # a workspace's: the app's client
rust-js build --check               # writes nothing; fails, naming each file, if one is stale
```

It runs once: Cargo checks each crate with the native compiler as its
workspace wrapper (ADR 0101), the one way an app's Rust is built (ADR 0374),
and each module's JS, its map and its `.d.ts` are written beside its Rust,
to be committed (ADR 0041): the dev server sees them as any edit, and Fast
Refresh keeps state. Then each of the workspace's crates' checks of its JS
run, a build's (ADR 0117). A compile error is rustc's, and nothing is
written. Cargo builds in `node_modules/.cache/rust-js/target`, out of what a
bundler watches, unless `CARGO_TARGET_DIR` says otherwise. An app's `build`
script checks before it builds: `rust-js build --check && next build`.

Distributed tooling supports Node.js and Bun using standard Node.js APIs. Bun
is used to develop, test, and create packages in this repository; users do not
need it to run installed packages. The native launcher uses the host's current
JavaScript runtime.

`@rust-js/build` exports nothing but its command: its modules are `rust-js
build`'s own, and the playground's, in this repository, which compiles its
own Rust with rust-js.wasm (ADR 0045). An app runs the command.

`/patch` tells an app's Cargo where
the crates its npm packages have are: each package with `"rust-js": { "crate" }`,
as `@rust-js/builtins` and `@rust-js/webapi` have, found as Node finds it, in
npm's, bun's or pnpm's layout, is a `[patch.crates-io]` entry in the app's
`.cargo/config.toml`, between `# rust-js: begin` and `# rust-js: end`, which
`rust-js build` and the editor's check, `rust-js-check`, write each time they
run Cargo. A crate installed twice, which would be two of each of its types, is
refused, with who asked for each.

`@rust-js/build` is on npm, released with the compiler (ADR 0120). A local
tarball can be made with `bun pm pack` from `tooling/`. `rust-js build` finds
the compiler as `RUST_JS_COMPILER` names it, then the application's
`@rust-js/native` package, then the development checkout; the compiler binary
isn't in `@rust-js/build`.

Build and package a native compiler for the current macOS or Linux host:

```sh
cargo build --release --locked
bun run pack:compiler target/release/rust-js /absolute/artifacts/native.tgz
```

The private `@rust-js/native` package contains the binary and a JavaScript launcher,
with OS/architecture restrictions in its package manifest. Install the pinned
Rust toolchain, its minimal profile and the `wasm32-unknown-unknown` target, on
the destination machine first. The launcher
asks that toolchain for its sysroot and sets the dynamic-library search path
before forwarding arguments and exit status to the compiler. It does not install
toolchains or modify global configuration. `rust-js build` finds this package
automatically; `RUST_JS_COMPILER` names another. It has no command of its own:
an app's is `rust-js build`.

This is a local packaging path, tested on the development host. It still depends
on Node.js or Bun, rustup, and compatible native system libraries. Clean-machine testing,
Linux/macOS version qualification and signing remain open;
the archive is not a standalone portable compiler distribution.

To assemble the packages together, with the crates an app's Cargo builds
with, as npm has them (ADR 0118), use a new output directory:

```sh
bun run pack:distribution target/release/rust-js /absolute/artifacts
cd /absolute/artifacts
shasum -a 256 -c SHA256SUMS
```

On Linux, `sha256sum -c SHA256SUMS` also works. The command checks package versions
against the compiler, stages every tarball, writes `distribution.json` with the
compiler identity, host platform/architecture and archive hashes, then renames
the completed directory into place. Existing destinations are rejected and a
failed build removes its staging directory. `SHA256SUMS` covers every archive
and the distribution manifest. These hashes detect corruption; they do not
authenticate the publisher. The command does not publish or install anything.

For a local installation, add the tarballs to the application's `package.json`
using paths relative to that file, then install with the application's package
manager; its `Cargo.toml` names the crates by version, and the patch tells
Cargo where they are:

```json
{
  "devDependencies": {
    "vite": "8.3.0",
    "@rust-js/build": "./artifacts/build.tgz",
    "@rust-js/native": "./artifacts/native.tgz",
    "@rust-js/react": "./artifacts/react.tgz",
    "@rust-js/webapi": "./artifacts/webapi.tgz",
    "@rust-js/builtins": "./artifacts/builtins.tgz"
  },
  "overrides": {
    "@rust-js/build": "./artifacts/build.tgz"
  }
}
```

```sh
rust-js build
```

The isolated package test installs offline with lifecycle scripts disabled,
repeats installation with a frozen lockfile, and runs the installed `rust-js
build` on a React app's Cargo package; the Vite and Next.js suites exercise
the real bundlers and Fast Refresh beside it. The installed command and
launcher are exercised under Node.js, with a failing stub for Bun on `PATH`
so a hidden dependency fails the test. The command uses a Node.js shebang;
Bun-only users can run `bun node_modules/.bin/rust-js build`. The builder
invokes the compiler with its own runtime.

The builder queries `rust-js --version-json` before a build: its `version`,
`toolchain`, and `abi`, matching the compiler identity in emitted manifests.
Cargo runs that toolchain, and `@rust-js/runtime` must be that version.
`rust-js --version` remains the human-readable form.

## Share model source with native Rust

Keep shared models in a crate of the workspace, which the client's crate and
the native server both depend on, or include one file from both:

```rust
#[path = "../shared/model.rs"]
mod model;
```

The shared module can contain supported Serde models and portable validation
functions. Keep native I/O in the server and browser APIs in the client. The
compiler manifest lists the loaded shared file in `sources`; after changing
it, run `rust-js build` for the client, and rebuild the native server.

[`test/shared-code.test.ts`](../test/shared-code.test.ts) creates an app outside
the repository, compiles the same models and validation for both targets, and
passes client-produced JSON through a native executable and back to the client.
It checks malformed requests and rebuilds both targets after changing a shared
validation rule.

`manifest.js` validates the compiler's version-1 build result and remaps structured
path fields. `publish.js` commits a completed WASI result to the host filesystem.
Callers supply only a successful compiler result and serialize builds targeting
the same outputs. Publication preserves unchanged files, checks stale-file
ownership, and rolls back ordinary I/O errors. It does not guarantee crash-atomic
multi-file replacement.

## Build a Cargo workspace

Cargo builds a workspace's libraries with rust-js as its workspace wrapper
([ADR 0101](../docs/decisions/0101-cargo-workspace-wrapper.md)), each to JS of
its own ([ADR 0100](../docs/decisions/0100-separate-crates.md)), as `rust-js
build --package frontend` asks for:

It runs `RUSTC_WORKSPACE_WRAPPER=<compiler> cargo check --target
wasm32-unknown-unknown`. Each library of the workspace that `frontend` uses,
and `frontend` itself, is JS in Cargo's target directory, beside the metadata
of that build of it, importing the others, then written beside its Rust. A
feature set built before is the JS it was.
Each file a crate's manifest lists is checked, of a build Cargo has as done
too: one gone or edited is refused, and `cargo clean -p <package> --target
wasm32-unknown-unknown` builds it again.
Registry crates, build scripts and procedural macros are built by rustc, as
Cargo asks; of registry crates, only serde's are known to rust-js at run time.
`cargo build` is refused.

A crate using React depends on the bindings in rust-js, and the React the
app has installed is the release it's checked for (ADR 0043), the latest
otherwise:

```toml
[dependencies]
react = { package = "rust-js-react", path = "/path/to/rust-js/react" }
```

In an app, `rust-js build --package frontend` builds the workspace, for the
React the app has installed, each module's JS beside its Rust, to be
committed (ADR 0041): `frontend/src/api.rs` is `frontend/src/api.js`. The
app imports the package's JS where it is:

```js
// web/src/main.jsx, Vite's root a directory of the workspace
import { App } from "../../frontend/src/lib.jsx";
```

A change to any crate of the workspace is one more `rust-js build`, and the
JS it changes is a Fast Refresh. Without rust-js, the app builds from what's
committed.
