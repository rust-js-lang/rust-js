# 0412. `rust-js build` is its own process

Status: Accepted. Supersedes [0380](0380-with-rust-js.md); amends
[0192](0192-next.md), [0101](0101-cargo-workspace-wrapper.md),
[0117](0117-output-hooks.md), [0118](0118-bindings-on-npm-only.md),
[0196](0196-typescript-declarations.md) and
[0045](0045-playground-on-vite.md); replaces rule 1 of
[0378](0378-one-file-at-a-time.md), a config that compiles, and delivers its
rule 4, a build that checks the committed JS.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): how an app's Rust is
compiled, not what a program means.

## Context

A Next.js app's config compiled its Rust, `withRustJs(nextConfig)`: `next
dev` compiled it and watched it, recompiling on each save, and `next
build` compiled it first; `rust-js-next compile` wrote the JS alone. A Vite
app's plugin, `@rust-js/vite-plugin`, did the same inside Vite, and served
its JS as `rust-js:<package>`. The compiler ran inside the host's process,
and the dev server's output was its and rustc's interleaved. A developer
who wants to say when the Rust is compiled, and to see its errors on their
own, had no way to. And each was another way to compile an app's Rust:
two plugins, a second command, the compiler's own CLI, an install's
`postinstall` writing Cargo's patch, a `declarations` key, and a save's
fewer checks than a build's.

## Decision

**An app's Rust is compiled one way, by one command, `rust-js build`, run
in the app's directory: once, its own process, beside whatever serves or
builds the app, Next.js's or Vite's, which knows nothing of rust-js. It
writes each module's JS, its map and its `.d.ts` beside its Rust, as
they're committed (ADR 0041). `rust-js build --check` writes nothing, and
fails, naming each file, where what's beside the Rust isn't what it
compiles to; `--package <name>` names a workspace's app package. There is
no other way.**

```bash
next dev                # one terminal, as Next.js runs it
npx rust-js build       # another, after a change to the Rust
```

```json
"scripts": {
  "dev": "next dev",
  "build": "rust-js build --check && next build"
}
```

- **One command**, `rust-js`, `@rust-js/build`'s, which has only `build`:
  the compiler is what it runs, `@rust-js/native`'s, which has no command
  of its own. `withRustJs`, `rust-js-next`, `@rust-js/vite-plugin` and
  its `rust-js:<package>` imports are gone: an app imports the JS where it
  is, `./App` or `../../frontend/src/lib.jsx`.
- **Declarations always** (ADR 0196): every module's `.d.ts`, but under a
  Next.js app's Pages Router (ADR 0276); the `declarations` key is an
  error that says so. The compiler is told by the one caller of it,
  Cargo's wrapper, `--declarations`, and finds `@rust-js/typescript`,
  `@rust-js/build`'s dependency, where `RUST_JS_TYPESCRIPT` says.
- **Cargo's patch** (ADR 0118) is written where Cargo runs, by `rust-js
  build` and the editor's check (ADR 0222): `rust-js-patch` and its
  `postinstall` are gone.
- **A crate's checks** (ADR 0117) run on each build, all of them: there's
  no save to run fewer for, and `when` is no key of one.
- **Once, not watching**: nothing recompiles on a save. Next.js's dev
  server sees the written JS as any edit, and Fast Refresh keeps state.
- **An error is rustc's, in `rust-js build`'s terminal**, which exits 1;
  the page keeps the JS that last compiled.
- **A build checks, not compiles**: what's committed is what's built, and
  `--check` says if that's stale. A teammate who doesn't write Rust builds
  from what's committed, with no compiler.
- **A Next.js app's Pages Router routes** still get no declarations (ADR
  0276): `rust-js build` asks the app's `package.json` whether it's a
  Next.js app.

## Why

- **A process of its own is the developer's to run**, and its output
  theirs to read: rustc's errors aren't mixed into Next.js's log, and a
  compile is never started by a config being loaded.
- **It's the same for every host**: an app's dev server is the host's
  own, unchanged, and its Rust is compiled the one way.
- **Tests**: `next.test.ts` builds the example with `rust-js build` then
  `next build`, checks it with `--check` before and after a change, and
  drives `next dev` in a browser while `rust-js build` runs beside it,
  each build a Fast Refresh, an error leaving the page as it was;
  `vite.test.ts` and `pilot.test.ts` do the same with Vite, a workspace's
  package by `--package`; `packages.test.ts` runs the installed command;
  and the architecture test holds the one way: no plugin, one command of
  one subcommand, a native package of no command.

## Consequences

- A save is no longer a refresh: the developer runs `rust-js build`.
- Tailwind reads the class names in the JSX rust-js writes, and an app
  leaves the `.rs` files out of its sources, `@source not "./**/*.rs";`,
  so an edit of one before it's built doesn't reload the page.
- The playground compiles its own Rust with rust-js.wasm by its own
  script, `bun compile-rust.ts`, beside Vite, as `rust-js build` is an
  app's (ADR 0045).
- `@rust-js/typescript`, and TypeScript, are in every app, as
  `@rust-js/build`'s.
- A root module that stops having JSX is `App.js`, not `App.jsx` (ADR
  0382): an importer names it `./App`, and Vite's dev server, which keeps
  where it found `./App`, is restarted to see the other file.
