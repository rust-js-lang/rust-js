# 0558. A byte's escape is its bytes

Status: Accepted. Extends [0341](0341-byte-slice-text.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): the bytes, the text
and the count Rust's `ascii::EscapeDefault` gives.

## Context

aho-corasick's `DebugByte` iterates `core::ascii::escape_default(b)`,
capitalizing its hex digits as bytes; that, and a byte's
`escape_ascii()`, which gives the same type, were refused. A byte slice's
`escape_ascii()` is its text (ADR 0341), whose bytes aren't iterated; a
single byte's escape is another type, `ascii::EscapeDefault`, an
iterator of `u8`.

## Decision

**`ascii::escape_default(b)` and `b.escape_ascii()` of a byte are std's
iterator over an array of its escape's bytes, a `$iter` of
`$escapeByte(b)`, which steps, counts, clones and steps from its back as
any is; `{}` of one is the bytes it has left as text, `$restBytes(it)`.**

- `$escapeAscii` is the one rule of an escape; `$escapeByte` asks it.
- `is_byte_escape` is the one answer to which type is one.

## Why

`byte_escape` iterates, counts, collects and shows the escapes of
printable, named, `\x7f` and high bytes, as `DebugByte` does, and shows
one partly stepped and steps its clone's back, against native Rust.

## Consequences

- Mutations refuse each call, take it for no array's iterator, refuse
  `{}`, and show all of one stepped.
