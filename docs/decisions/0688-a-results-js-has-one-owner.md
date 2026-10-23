# 0688. A `Result`'s JS, and an enum's, has one owner

Status: Accepted.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; the JS is the
same, byte for byte, across the corpus.

## Context

A review of the compiler's boundaries (2026-10-23) found two owners of
one representation. `lower/std_types/result.rs` said a `Result`'s JS,
`{ TAG: "Ok", _0: value }`, was "made, tested and read here alone". But
`prepare.rs`, which reads a `match` of a `$try`'s `Result` back into a
`try`, recognized `TAG === "Ok"`, `!== "Err"` and `._0` itself, and lowering
can't be asked from preparation, which takes no rustc.

Tracing it found more. Fifteen of lowering's reads spelled `"_0"`,
`"TAG"` or `"Ok"` themselves:

- **A `Result`'s**: its `into_iter`, its hash, `?`'s residual,
  `get_disjoint_mut`'s, its `{:?}`, an `Option`'s `ok_or` drop, tidy's
  `?`.
- **Another enum's first field**: a `Cow`'s, an `Entry`'s, a `VarError`'s,
  serde_json's `Value`'s.

The general shape had four copies of `format!("_{i}")`, a tuple variant's
field (`representation`, `declarations`, `aggregates`, `serde/de`), and
`bindings.rs` held the default `"TAG"`. Each is right alone. Two
spellings drift, though, and the bug lives between them.

## Decision

**An enum's JS shape is the owned tree's, `js::TAG` and
`js::positional(i)`, and a `Result`'s is `js::result`'s: `tagged`, `made`,
`ok`, `err`, `is`, `tag`, `held`, `made_ok`, `tested`, `reads_held`,
`unwrapped_made`. Lowering and preparation both ask them.**

- `js::result` needs no rustc, so preparation can ask it. Lowering keeps
  what does need rustc, `?`'s `Conversion` of the error and the `FnCx`
  methods that ask an `Option` or a `Result` of a type, in
  `lower/std_types/result.rs`.
- A variant's field is `representation::variant_field`, which reads
  `js::positional` for a tuple variant: the TypeScript declarations ask
  it too, where they re-derived it.
- Rust's own variant names are rustc's: a write's `Ok` is the `ResultOk`
  lang item, and serde's `{"Ok": ..}` key is the variant's own name.
- Held by `test/architecture.test.ts`: no file of lowering or of
  preparation spells `"TAG"`, a variant's `"_0"` or a `Result`'s `"Ok"` and
  `"Err"` in its code, and each owner does.

## Consequences

- Changing an enum's or a `Result`'s shape is a change to `js.rs` or
  `js/result.rs` alone, and the test finds a new spelling elsewhere.
- The runtime's helpers that make or read a `Result`, `$try`, `$settle`,
  the parsers and the rest, are written in JS and keep their own literal
  `{ TAG: "Ok", _0 }`. The corpus runs each of them, so a change to the
  shape that missed one would fail where it's read.
