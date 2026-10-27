# 0970. A `LinkedList`'s `into_iter()` owns its array

Status: Accepted. Extends [0316](0316-linked-list.md)'s list, an array as a
deque is, to the list's `IntoIter`. Fixes #377.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. Rust
drops what the iterator hasn't given where the iterator is dropped; the JS
must too.

## Context

A `LinkedList` is the array of its items, as a `VecDeque` is. Its
`into_iter()` wasn't one of the iterators that own an array's items
(`array_source`), so it was JS's iterator of the list. Its drop, std's
compiled one, read std's field of what that iterator isn't,
`for (const item of value.list)`, and threw `value.list is not iterable`
for any item with a destructor. `ll.into_iter().next()`, `find(..)` and
`rev()` crashed or were refused (`rev()`: "can't step from its back").

## Decision

`std::collections::linked_list::IntoIter` is an owning array source, as
`vec_deque::IntoIter` is: `$iter` of the list's array, whose drop drops what
it hasn't given, from its front or its back, and whose back is the array's.

A `collect()` into a `LinkedList` asks no room first: std's `FromIterator`
of a list makes a new one and extends it, never `SpecFromIter`'s capacity
check (ADR 0940's `$roomForHint` is a `Vec`'s).

## Consequences

`test/corpus/linked_list_into_iter.rs` checks the drops of `next()`,
`find(..)` and `rev().next()` against native Rust.
`linked-list-into-iter-not-owning` is its mutation.
