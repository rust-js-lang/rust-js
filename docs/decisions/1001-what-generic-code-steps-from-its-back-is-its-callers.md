# 1001. What generic code steps from its back is stepped of its callers' types

Status: Accepted. Extends [0545](0545-a-map-stepped-from-its-back-is-made-knowing-it.md).
Fixes #388.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. The
program threw where Rust steps from the back.

## Context

```rust
fn back<I: DoubleEndedIterator<Item = u32>, P: FnMut(&u32) -> bool>(mut it: std::iter::Filter<I, P>) -> Option<u32> {
    it.next_back()
}
back(v.iter().copied().filter(|x| x % 2 == 1))
```

The back-stepped analysis found `Filter<I, P>` stepped from its back, of
`back`'s own type parameters. The caller makes
`Filter<Copied<Iter<u32>>, {closure}>`, which it never named, so the
filter was JS's `it.filter(p)`, with no back: `TypeError: it.back is not
a function`, where Rust gives `Some(5)`.

## Decision

The analysis keeps what each body steps from its back by its function.
What holds a type parameter is carried to each call of the function:
substituted with the call's types, it's what that caller steps, and
through what calls that caller in turn. It stops after 32 calls deep, as
a function calling itself with a type made of its own types would go on.
A caller's own type that looks like its callee's, `Filter<I, P>` of both,
is checked in the caller's set, not the crate's.

## Consequences

`test/corpus/adapters_given_to_generic_backs.rs` checks a filter and a map
given to generic code, directly and through another generic function,
against native Rust. The mutation is `generic-back-steps-uncarried`.

The probing found two of #382's survivors dead, now that a held `Zip` is
std's struct (ADR 0997):
- `iterator_steps`' rule asking an uncompiled adapter's impl what it asks
  of what it holds. Every such impl asks the same trait.
- `impl_asks`' looking through a reference.

Both go with their mutations, `adapter-impl-asks-unread` and
`adapter-impl-asks-of-references`.
