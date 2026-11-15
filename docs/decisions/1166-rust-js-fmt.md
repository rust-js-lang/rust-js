# 1166. `rust-js fmt` is the one formatter

Status: Accepted (2026-11-15, the lead's approval of #1901's design). Step
A: the command. Step B, `jsx!` bodies wrapped at rustfmt's width, follows
as an amendment, after its preview of React's port.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. Formatting
changes where a program's text is, never what it does.

## Context

An app formatted its Rust with `cargo fmt`, which leaves a `jsx!` body as
it's written, and had no supported way to lay the body out: the
compiler's `--format-jsx` read one file and indented its bodies, and this
repository's `bun run fmt` glued the two together itself. #1901 asks for
one documented command, a `--check` that fails, stdin for an editor, and
JSX wrapped at a width.

Running rustfmt first showed one more thing: rustfmt can't read a `jsx!`
body, so it moves each of its lines with the call, a multi-line string's
second line too, `{"first\n second"}`'s, which changes the program.

## Decision

**`rust-js fmt [--check] [--stdin] [paths..]`** (`src/source_format.rs`,
the tooling CLI's `rust-js fmt` the compiler's):

- No paths: each target's module tree of the Cargo workspace it's run in,
  its roots by `cargo metadata` and each `mod name;` they declare by
  their tokens, as rustfmt finds them; a file no target declares is left
  as it is. Paths: those files alone.
- rustfmt runs where each file is, so the project's toolchain and
  `rustfmt.toml` are its, given its package's edition; on all the
  machine's cores.
- **Each `jsx!` body is then as it was written**, by its tokens, before
  the JSX pass lays it out, which changes only leading whitespace and
  never a literal's inside.
- `--check` writes nothing, prints each changed file's unified diff, and
  exits 1; every file is read and formatted before any is written, so one
  that can't be leaves them all as they were.
- `--stdin` formats stdin to stdout, rust-analyzer's
  `rustfmt.overrideCommand`.
- **`--format-jsx` is refused, naming `rust-js fmt`**: one way.
  `bun run fmt` and `fmt:check` run it, the workspace and then this
  repository's crates no target declares.

## Consequences

- This repository's `bun run fmt:check` passes as before: its module
  trees are rustfmt's 601 files, checked against `rustfmt -v`'s list.
- test/format.test.ts: its rows run `rust-js fmt --stdin`; a row of the
  refusal, `--check`'s diff and exit code, the moved literal kept byte for
  byte, and a workspace's module tree formatted, a commented and a quoted
  `mod` and a loose file left alone.
- Guide: react/jsx.md's Formatting, project/building.md.
- Step B, wrapping, as approved: oxc_formatter given the body's JSX with
  a placeholder for each piece of Rust, one batched rustfmt of the pieces,
  two rounds at most, each piece back byte for byte, a `//` comment kept
  or the run refused naming its line, and the playground's mirror after.
