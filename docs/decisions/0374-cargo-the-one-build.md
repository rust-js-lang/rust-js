# 0374. An app's Rust is built with Cargo, its one way

Status: Accepted. Amends [0084](0084-owned-phases-and-host-boundaries.md),
[0101](0101-cargo-workspace-wrapper.md), [0103](0103-runtime-package.md),
[0105](0105-create.md), [0114](0114-app-cargo-toml.md),
[0115](0115-binding-crates-on-crates-io.md), [0117](0117-output-hooks.md),
[0118](0118-bindings-on-npm-only.md) and [0120](0120-first-npm-release.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the JS an app's Rust
becomes is the same; only how a host builds it changes.

## Context

The Vite plugin built an app's Rust three ways: a crate root at a time,
`crates: ["src/App.rs"]`, with metadata the builder made from
`@rust-js/resources` for the bindings it was told of, `bindings: ["react",
"serde"]`; a Cargo package, `cargo: { package }` (ADR 0101); and a function
of the caller's, the playground's. The first was what a created app no
longer used (ADR 0105), and it had what the second lacked: a crate's checks
(ADR 0117) ran only there, and each found its committed JS its own way.
AGENTS.md's first rule is exactly one way to do each job.

## Decision

- **An app's Rust is a Cargo package**, built by the builder's Cargo
  session, Vite's and Next.js's alike: `rustJs({ cargo: { package } })`.
  `crates`, `bindings`, `externs`, `resources` and `rustcFlags` are gone,
  with the builder's single-crate `compile`, its React and serde recipes,
  and the `@rust-js/resources` package. Its crates are Cargo's
  dependencies, the bindings npm's (ADR 0118), which the distribution
  carries now in its place.
- **A crate's checks run in Cargo's build**, by its driver, as ADR 0117
  said they would: once Cargo is done, the session runs each of the
  workspace's crates' checks of its JS as served, `rust-js --checks <root>
  --hooks save|build <file>..`, a dev server's a save's.
- **Where Cargo builds is the session's to say**: `CARGO_TARGET_DIR`, or
  `node_modules/.cache/rust-js/target`, out of what a bundler watches;
  Tailwind took Cargo's files in `target/` for new ones to scan, and
  reloaded the page, as Turbopack took each for a change.
- **Without a compiler, what's said is one function's**, `withoutCompiler`,
  of the committed JS the session finds beside the package's root.
- **The playground's `compile` stays its own**: it builds its Rust with
  rust-js.wasm, the compiler it ships, and is no app's.

## Consequences

- The Vite tests are Cargo apps, `cargoApp` in `test/support.ts`; their
  checks run under a save and a build as before.
- A distribution is the packages and the crates: `packages.test.ts` builds
  a Cargo app of them, offline, through the installed plugin.
