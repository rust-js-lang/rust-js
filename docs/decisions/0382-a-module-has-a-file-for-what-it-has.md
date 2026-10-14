# 0382. A module has a file for what it has, the root too

Status: Accepted. Amends [0019](0019-one-js-file-per-module.md); delivers rule 2
of [0378](0378-one-file-at-a-time.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): which files are
written, not what a program means.

## Context

A module without functions writes no file (ADR 0019), but the crate's
root always wrote one. An app's root is its list of files,
`#[path = "components/ExternalLink.rs"] pub mod external_link;`, and
nothing else: react.dev's first Rust file came with a `src/lib.js` of a
header and a source map's comment, a file nobody wrote and nothing
imports. And a module whose only item is `js::import!("./styles.css")`
wrote no file, its import lost without a word.

## Decision

**A module has a JS file for what it has: an item, a `js::import!`, a
re-export or a default. The root is no other: a root of `mod`s alone
writes none.**

- The Vite plugin's entry is a root that has its `App`, so it has its file.
- Without a compiler, the Next.js plugin builds the JS committed beside
  the Rust, the root's or, for a root of `mod`s alone, its modules', and
  says so.

## Consequences

- An app's commit of a file moved to Rust is that file's JS, not a
  `lib.js` beside its root.
- A module of a `js::import!` alone imports what it says.
