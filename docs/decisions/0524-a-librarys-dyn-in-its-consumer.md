# 0524. A library's `dyn` in its consumer

Status: Accepted. Extends [0049](0049-traits-and-generics.md) and
[0100](0100-separate-crates.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): a `dyn` is its
pair whichever crate holds it; only which crate may hold one changes.

## Context

A `dyn` of a rust-js trait is a pair, `{ value, impl }` (ADR 0049), and
`is_pair_trait` says a library's trait is one too. But whether a crate
may hold a value of a `dyn` asked `id.is_local()`: a consumer of a
library's `Box<dyn Shape>` was refused ("values of type `dyn dep::Shape`")
though the library made the pair, with its dictionary. Found accepting
[0523](0523-libraries-share-one-counted-rc.md), by a library's
`Box<dyn Show>` of an `Rc`.

## Decision

**A `dyn` of a library's trait is supported where its own crate's would
be**: `is_rust_trait`, the crate's own or a library's, the one answer
`is_pair_trait` asks, and the trait `dyn_supported`.

## Why

`two crates: dyn_trait` calls, sums and drops a library's `dyn Shape`s,
the library's own types' and one of the consumer's, against native Rust.

## Consequences

A mutation asks `is_local` again.
