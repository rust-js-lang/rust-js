# 0983. A `fuse()` that drops its source stays where it's stepped

Status: Accepted. Amends [0561](0561-std-bodies.md)'s and
[0475](0475-a-generic-into-iter-is-the-iterator-concrete-code-has.md)'s
`stepped_in_place`: a compiled `fuse()` stepped where it's made is left
out only where it's given no drops. Found by #382's triage.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. Rust
drops what the iterator didn't give where it's dropped; the JS didn't.

## Context

```rust
let c = louds(3).into_iter().fuse().map(|l| l.0).next();
```

Native Rust drops item 0 as the `map` takes it, then items 1 and 2 as
the iterator is dropped, at the end of the `let`. The JS dropped item 0
alone. A `fuse()` stepped where it's made, std's compiled
`fuse(src, IIterator, IFused, dropSelf, dropSelfItem)`, was replaced by
`src`, since a fuse adds nothing to what's stepped till it ends. But
`dropSelf`, the drop of what the `Vec`'s `IntoIter` didn't give, went
with it, so `src` was a bare array iterator nothing closed. `rev()` of
the same lost them too.

## Decision

The fuse is left out only where it's given no drops: its dictionary and
its fact alone, the drops it doesn't use left off (ADR 0300). One given
its source's drop stays, and drops what it didn't give as it's closed.

## Consequences

`test/corpus/owned_items_through_fuse.rs` checks items with a destructor
through a `fuse()`, an `enumerate()`, a `rev()`, `nth` and a generic
adapter, against native Rust. No other case changes. The mutation is
`fuse-of-drops-elided`. The case also catches #382's survivor
`enumerate-back-source-unclosed`.

`louds(4).into_iter().fuse().enumerate().rev()`, made in place, is still
refused (#383).
