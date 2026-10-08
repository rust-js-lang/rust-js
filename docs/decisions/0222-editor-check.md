# 0222. An editor checks the app through rust-js

Status: Accepted. Extends [0101](0101-cargo-workspace-wrapper.md) and
[0113](0113-plain-rustc.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A plain rustc compiles an app but what's inside JSX (ADR 0113): `jsx!` is
an `Element` it doesn't look inside. An editor's rust-analyzer checks with
it, so in every component what only JSX uses, a variable, an import, a
component, was unused, a type error in JSX wasn't said, and a component
only `js::export_default!` exports was dead code. react.dev's `Heading`
showed dozens of warnings, none true.

## Decision

**`rust-js-check`, of `@rust-js/build`, is the app's `cargo check` through
rust-js, as its dev server's, Cargo's JSON printed as it comes: what
rust-analyzer runs as its check, which an app made by `@rust-js/create`
says in `.vscode/settings.json`:**

```json
{
  "rust-analyzer.check.overrideCommand": ["./node_modules/.bin/rust-js-check"]
}
```

- **It's the dev server's check**, rust-js as Cargo's workspace wrapper
  (ADR 0101), so what it says is what a build says: inside JSX, what's
  used is used, and what's wrong is an error where it's written.
- **In a target directory of its own**, `node_modules/.cache/rust-js/editor`,
  it neither waits for the dev server's check nor makes it check again,
  and Turbopack and Vite don't watch it. It writes no JS in source.
- **A function `js::export_default!` names is used**, by JS: rust-js marks
  it `#[allow(dead_code)]` as it reads the module.

## Why

- **The editor says what the build says**: no warning a build doesn't
  have, and the errors JSX has.
- **It's rust-analyzer's own setting**, a check command, as Clippy's is;
  rust-js asks nothing more of it.
- **It's tested**: a check of an app whose component uses a variable only
  in JSX, exported by `js::export_default!`, says nothing; one with a type
  error in JSX says it, at its line of the `.rs` file; a made app has the
  setting.

## Costs

- **rust-analyzer's own completion and hover still don't look inside
  `jsx!`**: only its check is rust-js's.
- **A check takes as long as the dev server's**, a rust-js compile of the
  crate, not rustc's.
