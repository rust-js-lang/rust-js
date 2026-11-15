# 1166. `rust-js fmt` is the one formatter

Status: Accepted (2026-11-15, the lead's approval of #1901's design). Step
A: the command. Amended, step B: each `jsx!` body laid out at rustfmt's
width, approved by the lead for Tu on its preview.

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

## Amendment: each `jsx!` body laid out at rustfmt's width (step B)

**A `jsx!` body is laid out as JSX is, by oxc's rules, Prettier's, at
rustfmt's `max_width` less its indent, rustfmt's `tab_spaces` its indent**
(`src/source_format/wrap.rs`; oxc's side `format::jsx_expression`; the
body's shape `jsx_syntax::formatting::shape`, read from what the
compiler's parser accepted):

```text
  body ──shape──► JSX of placeholders ──oxc──► laid out ──splice──► body
                    │                                      ▲
                    └── each piece of Rust ──rustfmt───────┘
```

- Its props one a line where they don't fit, `>` on a line of its own,
  each child on one, as #1901 asked; Prettier's rule stays that an
  element of two props or more, a tag child or two expressions puts its
  children on lines of their own (the lead's decision).
- **Each piece of Rust is rustfmt's**, all of a body's in one run, at the
  width it gets: a `{expr}`, a prop's value, a spread. A `jsx!` in one is
  laid out first, at what's left of the width. A piece is a placeholder
  as wide as it is on one line; one of several lines a two-line template
  literal, which oxc keeps as it is and hugs in its braces, its parent
  broken, `onClick={move |_| {` .. `}}`; a child of several lines wider
  than any line, so it's on one of its own, its braces oxc broke joined
  again. A piece still too long where it lands is laid out again at the
  width its column leaves: two rounds at most.
- **Each piece comes back as it is, once, or the run fails**: a
  placeholder oxc dropped or doubled is an error. The placeholders' stem
  is one the body holds none of, `Ω`, else `ΩΩ`, so no name or comment
  is refused (the lead's decision). A literal comes back as it's
  written; a line inside a token, a multi-line string's or a comment's, is
  never indented or unindented; a piece's lines go to rustfmt relative to
  its own line, so an opaque macro in one stays put.
- **Two guards on every run, not only in tests**: oxc's output has its
  input's tokens, none added (`{" "}`) or changed; and the body's
  literals are the same before and after. Either failing leaves the file
  as it was, naming its line.
- **A comment no JSX slot holds**, a `//` between props or children,
  keeps its body's lines, only its indentation laid out, and `--check`
  says so on stderr, naming the line; a `{/* .. */}` child is laid out
  as it's written.
- The edition of a package of `edition.workspace = true` is its
  workspace's.

What holds it: test/format.test.ts (#1901's example exactly; a path tag,
a spread, a multi-line closure, a `match`, a comment child, a nested
`jsx!` and a multi-line string; `Ω` in a name and a comment; a long
child; a piece laid out again; bodies the visitor meets out of order; an
opaque macro; todomvc twice; the comment's note); every `jsx!` file of
the playground, the examples and next formatted twice the same, and the
playground's, todomvc's and react_counter's JS the same compiled before
and after; React's port unchanged. Mutations of `wrap.rs` (each rule
above) and `fmt-foreign-jsx-kept`.
