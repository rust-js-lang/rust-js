# 0556. A `copied()` given back as an `impl Iterator` is what it copies

Status: Accepted. Extends [0547](0547-a-copied-iterator-with-a-destructor-is-closed.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): each item a copy
where it's changed, as Rust's are.

## Context

regex-syntax's `ascii_class` gives back `slice.iter().copied()` as an
`impl Iterator<Item = (u8, u8)>`, which was refused: "`Iterator::copied`,
from its MIR". `is_its_source`, the one answer to which adapters are
their source in JS (ADR 0547), read the call's output type, the opaque
`impl Iterator`, which it didn't see through.

## Decision

**`is_its_source` sees through an `impl Iterator` to the adapter it is.**
A `copied()` given back so is the iterator it copies, `$iter(slice)`;
an item changed where it's used is copied there, as any is.

## Why

`copied_given_back` collects, maps and changes a copied tuple of an
`impl Iterator` a function gives back, the static slice unchanged,
against native Rust.

## Consequences

- regex-syntax compiles; regex stops at aho-corasick's unaligned reads,
  addressable memory, which its draft's gates are about.
- A mutation leaves the `impl Iterator` unseen.
