# Build hosts

`build.js` is what a host builds an app's Rust with: Cargo, which checks each
crate with the native compiler as its workspace wrapper (ADR 0101), the one
way an app's Rust is built (ADR 0374). Vite's and Next.js's plugins use it;
another host can too, without importing either.

Distributed tooling supports Node.js and Bun using standard Node.js APIs. Bun
is used to develop, test, and create packages in this repository; users do not
need it to run installed packages. The native launcher uses the host's current
JavaScript runtime.

```js
import { createNativeBuilder } from "@rust-js/build/build";

const builder = createNativeBuilder({ root: "/absolute/app" });
const session = builder.cargoSession({ manifestPath: "/absolute/app/Cargo.toml", packageName: "app", inSource: true });
const built = await session.build({ save: false, warn: console.warn });
// Again when what it read changes:
if (session.reads(changedFile)) await session.build({ save: true, warn: console.warn });
```

A session's `build` is `cargo check` of the package, then each of the
workspace's crates' checks of its JS (ADR 0117), a save's or a build's; its JS
is beside each module's Rust, to be committed (ADR 0041), unless `inSource` is
`false`. `reads(file)` says whether a change to `file` is one to build again
for, `sources()` what the last build read, to watch, and `committed()` the JS
committed beside the package's root, which a host builds from without a
compiler. Cargo builds in `node_modules/.cache/rust-js/target`, out of what a
bundler watches, unless `CARGO_TARGET_DIR` says otherwise.

`@rust-js/build` exposes package entry points: `/build` for the builder,
`/cargo` for Cargo's checks, `/manifest` for build-result validation, and
`/publish` for WASI artifact publication. Vite and the playground declare this
package dependency; neither imports tooling through a path outside its own
package.

`/patch`, and its command, `rust-js-patch [app-dir]`, tell an app's Cargo where
the crates its npm packages have are: each package with `"rust-js": { "crate" }`,
as `@rust-js/builtins` and `@rust-js/webapi` have, found as Node finds it, in
npm's, bun's or pnpm's layout, is a `[patch.crates-io]` entry in the app's
`.cargo/config.toml`, between `# rust-js: begin` and `# rust-js: end`. An app
runs it as its `postinstall`, so Cargo and its editor have them once it's
installed. A crate installed twice, which would be two of each of its types, is
refused, with who asked for each.

Both host packages are on npm, released with the compiler (ADR 0120). Local
tarballs can be made with `bun pm pack` from `tooling/` and `vite-plugin/`.
Compiler discovery first uses the supplied `rustJs` path, then the application's
`@rust-js/native` package, then the development checkout; compiler binaries are
not included in these host packages.

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
toolchains or modify global configuration. The builder discovers this package
automatically. To override it, set `rustJs` to an absolute compiler or launcher
path, including its installed `.bin` link.
The host hashes and watches both the launcher and its compiler binary.

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
    "@rust-js/vite-plugin": "./artifacts/vite-plugin.tgz",
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

```js
rustJs({ cargo: { package: "app" } })
```

The isolated package test installs offline with lifecycle scripts disabled,
repeats installation with a frozen lockfile, and builds a React app's Cargo
package through the plugin. It omits Vite's peer for its direct hook test; the
separate Vite suite exercises the real bundler and Fast Refresh. The installed
launcher and host are exercised under Node.js, with a failing stub for Bun on
`PATH` so a hidden dependency fails the test. The CLI uses a Node.js shebang;
Bun-only users can run `bun node_modules/.bin/rust-js`. The builder invokes it
with its own runtime.

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
compiler manifest lists the loaded shared file in `sources`, so a session
builds the client again when it changes; rebuild the native server separately.

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
its own ([ADR 0100](../docs/decisions/0100-separate-crates.md)):

```js
import { checkCargo } from "@rust-js/build/cargo";

const { js, crates } = await checkCargo({
  manifestPath: "Cargo.toml", toolchain: "<pinned release>", compiler: "/path/to/rust-js",
  packageName: "frontend", features: [],
});
```

It runs `RUSTC_WORKSPACE_WRAPPER=<compiler> cargo check --target
wasm32-unknown-unknown`. Each library of the workspace that `frontend` uses,
and `frontend` itself, is JS in Cargo's target directory, beside the metadata
of that build of it, importing the others; `js` is `frontend`'s, and `crates`
each crate's JS and manifest. A feature set built before is the JS it was.
Each file a crate's manifest lists is checked, of a build Cargo has as done
too: one gone or edited is refused, and `cargo clean -p <package> --target
wasm32-unknown-unknown` builds it again.
Registry crates, build scripts and procedural macros are built by rustc, as
Cargo asks; of registry crates, only serde's are known to rust-js at run time.
`cargo build` is refused.

A crate using React depends on the bindings in rust-js, and `checkCargo`'s
`react` is the release it's checked for (ADR 0043), the latest otherwise:

```toml
[dependencies]
react = { package = "rust-js-react", path = "/path/to/rust-js/react" }
```

In Vite, the plugin's `cargo` option, its one way, builds the workspace with
a session, for the React the app has installed, and the app imports the
package's JS as `rust-js:<package>`:

```js
// vite.config.js, in the workspace's web/
plugins: [rustJs({ cargo: { package: "frontend", manifestPath: "../Cargo.toml" } }), react()]
```

```js
import { App } from "rust-js:frontend";
```

An edit to any crate of the workspace is one `cargo check`, and the JS it
changes is a Fast Refresh. Each module's JS is written beside its Rust, to be
committed (ADR 0041): `frontend/src/api.rs` is `frontend/src/api.js`. Without
rust-js, Vite builds from those. `cargo: { inSource: false }` leaves the JS in
Cargo's target directory only, which must then be one Vite serves from.

## Experimental Cargo planning

`@rust-js/build/cargo` exports `planCargoLibraries({ manifestPath, toolchain, target,
packageName?, features?, noDefaultFeatures? })`. Supply an exact toolchain pin and
an explicit target triple. `packageName` selects a member of a virtual workspace.

The planner invokes Cargo metadata with `--frozen`: create and commit the lockfile
first. It resolves offline without changing that lockfile, and returns local
library packages in dependency order, preserving dependency aliases, source paths,
editions and Cargo-resolved features. Development dependencies are excluded.
Registry/git packages, build scripts, procedural macros and packages without an
ordinary library target are rejected when reachable from the selected library.

This is dependency discovery, not a Cargo compilation entry point. It does not
produce rustc invocations, compile dependencies to JS, establish a cross-crate ABI
or cache compilation artifacts. Metadata feature sets are Cargo resolution data,
not a substitute for Cargo compiler-unit/build-script information. The native
builder and Vite do not consume this experimental plan yet.

The compiler also has an experimental scalar linkage proof, described in
[ADR 0085](../docs/decisions/0085-scalar-library-linkage.md). A producer uses
`rust-js shared.rs -o shared.js --library --manifest shared.json`; a consumer
passes `--dependency shared.json` plus real rustc `--extern` metadata after `--`.
This supports only the documented scalar free-function ABI. The caller must build
matching metadata and JS and rebuild dependencies after source edits. The Cargo
planner, native build adapter and Vite do not orchestrate these steps yet.
