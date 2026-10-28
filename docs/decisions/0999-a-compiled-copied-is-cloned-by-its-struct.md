# 0999. A `copied()` std compiles held is cloned by its struct

Status: Accepted. Amends [0525](0525-a-cloned-map-is-made-again.md)'s
amendment for a clone of a `copied()` (633ae1dea).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. The
`--std-iterators` build threw.

## Context

633ae1dea made a clone of a `copied()` of the crate's own iterator clone
what it copies: `copied.with({ ...copied.of })`. That holds where a
`copied()` is its source in JS, the normal build. With `--std-iterators`,
a `copied()` a struct holds is std's compiled `Copied`, whose clone is its
compiled `Clone`:

```rust
#[derive(Clone)]
struct Holder<I> { it: I }
let other = Holder { it: refs().copied() }.clone();
```

There it wrote `held.it.with(..)` of `iterator_copied(..)`: `TypeError:
held.it.with is not a function`. Main's forms corpus failed on
`copied_clone_of_its_own`.

## Decision

`clone_value` reads a `copied()` as its source only where it isn't held
compiled (`is_held_compiled`). A compiled `Copied` takes its compiled
`Clone`, as the other compiled structs beside it do.

## Consequences

`copied_clone_of_its_own` runs in the forms test's default list too, so
a mutation run reaches it. The mutation is
`compiled-copied-cloned-as-source`.
