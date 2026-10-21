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

## Amendment: `{:x?}` of a composite (2026-10-22)

`{:x?}` of an array, a `Vec`, a tuple, an `Option`, or a struct or enum
whose `Debug` is derived shows every integer inside it in hex, as Rust's
`Formatter` carries the flag to each part's `fmt`. The flag travels as a
width does: a composite given it takes the options route (`with_options`),
where each integer leaf shows its hex and each other leaf, a string, a
float or a `bool`, ignores it, as std's `fmt` of it does. A writer given
the options at run time, a derived `Debug`'s, reads `options.debugHex`:
in a crate that asks `{:x?}` of anything (`analysis::uses_debug_hex`),
an integer it shows is `$debugInt(String(x), hexDigits, options)`, its
decimal or its hex, upper-case for `{:X?}`, after `0x` where it's
alternate, as `{:#x?}` shows it. A crate that never asks it writes what it
wrote. `test/corpus/debug_hex_composite.rs` holds each against native
Rust.
