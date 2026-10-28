# 0988. `$count` asks its iterator's `count` method, not a field

Status: Accepted. A wrong answer fixed, found by #382's triage of
`enumerate-clone-count-ignored`.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. The
program threw where Rust counts.

## Context

```rust
fn counted<I: DoubleEndedIterator>(mut it: I) -> (bool, usize) {
    let had = it.next_back().is_some();
    (had, it.count())
}
counted(v.iter().enumerate())
```

An `enumerate()` made where it's given to code that steps its back is
`$enumerate($iter(v))` ([0528](0528-fuse-and-enumerate-from-either-end.md)).
It keeps std's `Enumerate`'s fields, `iter` and `count`, how many it's
given, so std's compiled `Debug` and `Clone` read it as they read the
struct. `$count`, `count()` of a generic one, took any `count` it found for
the iterator's own method: `it.count(drop)` called a number, `TypeError:
it.count is not a function`, where Rust gives `(true, 2)`.

## Decision

`$count` calls `it.count` only where it's a function, a `$Steps`' own or a
compiled struct's `count` that `$stdIterator` gives. A `count` that's a
field is counted as any iterator is.

`$enumerate` no longer takes a starting count, which no caller gave: a
clone of it is std's compiled `Clone`, which reads its fields. Its
mutation, `enumerate-clone-count-ignored`, could not be caught, and is
removed.

## Consequences

`test/corpus/enumerate_counted_by_generic_code.rs` checks it against
native Rust, with the mutation `count-field-called`.
