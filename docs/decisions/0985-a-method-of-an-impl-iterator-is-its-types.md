# 0985. A method of an `impl Iterator` is asked of what it is

Status: Accepted. A gap closed: `mir_iter_method` asks every std
iterator method of its receiver's type revealed, as several of its
helpers already did. Found by #382's triage.

Case ([0262](0262-when-rust-and-js-disagree.md)): a gap, valid Rust
refused, and, of `collect()`, results change where they could: std asks
the room a compiled struct's hint gives (ADR 0940) whether or not it's
named behind an `impl`.

## Context

```rust
fn copied(v: &[u32]) -> impl ExactSizeIterator<Item = u32> + '_ {
    v.iter().copied()
}
copied(&v).len()
```

was refused: "`len()` of a lazy or generic `impl ExactSizeIterator`".
The same function returning `std::iter::Copied<std::slice::Iter<u32>>`
compiled. `knows_its_length` revealed the type, but the `len()` rule,
`has_a_back`, and the rest of `mir_iter_method` were asked of the `impl`
itself.

## Decision

`mir_iter_method` reveals its receiver's type once, where it reads it:
each method's rule is asked of what the iterator is.

## Consequences

Three snapshots change:

- `returned_impl_ends`: a reversed range returned behind an `impl` is
  stepped by `$rangeNextBack`, as a named one is.
- `generic_chain_of_into` and `lazy_iterator_clone`: a `collect()` of
  std's compiled struct returned behind an `impl` asks the room its hint
  gives, `$roomForHint`, as one named whole does (ADR 0940).

`test/corpus/backs_given_behind_impls.rs` checks what's given behind an
`impl` or a bound and stepped from its back against native Rust. The
mutation is `iter-method-type-unrevealed`; the case also catches #382's
`associated-double-ended-unknown`.
