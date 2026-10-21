# 0559. `{:x?}` of an integer is its hex

Status: Accepted. Extends [0165](0165-other-fmt-traits.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): the digits Rust's
`Debug` shows with its hex flag.

## Context

aho-corasick shows a transition's byte with `{:X?}`, which was refused:
"`{:x?}`". Rust's `Debug` of an integer under `{:x?}` or `{:X?}` is its
`LowerHex` or `UpperHex`, with the placeholder's width and `#`.

## Decision

**`{:x?}` and `{:X?}` of an integer are `{:x}` and `{:X}` of it, the one
way rust-js writes a radix.** `Spec::debug_hex` says which. Of anything
else, whose integers inside would each be hex, it's refused, naming its
type.

## Why

`debug_hex_integers` shows bytes, a `u32`, a negative `i8`, `{:#x?}` and
a width, and a crate's `Debug` that writes `{:X?}`, against native Rust.

## Consequences

- Mutations swap the cases, and refuse an integer's.
