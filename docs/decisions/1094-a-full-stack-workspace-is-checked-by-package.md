# 1094. The editor checks a full-stack workspace's client through rust-js and the rest natively

Status: Accepted (the user's decision, 2026-11-05, #788). Amends
[0222](0222-editor-check.md). Extends [0360](0360-the-app-is-a-program.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)). This changes only how
an editor checks an app. It changes no output.

## Context

ADR 0222's `rust-js-check` is the app's `cargo check` through rust-js:
every member of its workspace, for rust-js's target. That rests on every
member being a crate rust-js compiles. A full-stack app's isn't: hearth's
and cross-agent-message's workspaces hold a native server, axum and tokio,
beside the page rust-js compiles. `rust-js build --package frontend` and
`rust-js clippy --package frontend` build only the page; the editor's check
took no `--package`, so it checked the server for `wasm32-unknown-unknown`
too, where tokio's `mio` stops with 48 errors. rust-analyzer showed those
and nothing true. Without the setting, its plain `cargo check` can't see
inside `jsx!`: cross-agent-message's dashboard showed 21 false warnings.
So a full-stack app had no correct editor check either way (#788).

## Decision

**`rust-js-check` takes `--package <name>`, as `rust-js build` and
`rust-js clippy` do. The packages it names are checked through rust-js, as
before; every other member of the workspace is checked natively, by a plain
`cargo check --all-targets`, as rust-analyzer's own check would.**

```json
{
  "rust-analyzer.check.overrideCommand": ["./node_modules/.bin/rust-js-check", "--package", "frontend"]
}
```

- **One flag, as the other commands'.** `--package` is what `build` and
  `clippy` take for the same packages, so an app's scripts and its editor
  name its client the same way.
- **The rest is native, as rust-analyzer's own check.** Every target of
  each other member, with the workspace's toolchain and target directory,
  as a `cargo check` in it has them. The server's errors and warnings show
  where they are, as without rust-js.
- **One after the other.** rust-js's check runs first, then the native one,
  each printing whole lines of Cargo's JSON to one stdout. The exit code is
  the first that failed.
- **A crate both use is checked twice**, through rust-js as the client's
  dependency and natively as a member. A warning the two say alike is one
  diagnostic: rust-analyzer shows identical ones once, as it does for a
  crate's library and its tests.
- **Without `--package`, nothing changes**: an app whose every member
  rust-js compiles, a created app's, is checked through rust-js whole.

## Why

- **The editor says what the builds say**, of both sides: the client's as
  `rust-js build` reads it, inside `jsx!`, and the server's as `cargo
  build` does.
- **The workspace says no more than it did.** Which members are the client
  is the app's to say, on the command line, as it says it to `build`; no
  new key in `Cargo.toml`, and no guess from a member's dependencies.

## Costs

- **The settings name the client.** A full-stack app's
  `.vscode/settings.json` lists its rust-js packages, as its `build`
  script does.
- **A check of both takes both checks' time**, one after the other.

## Tests

`test/editor-check.test.ts`: a workspace of a rust-js client and a native
server that only a native target compiles, Unix's file modes, checked with
`--package client`, says only the server's one warning, at its line, and
succeeds; a type error in the server is said, there, and fails the check;
an argument the check doesn't take is refused with its usage.
