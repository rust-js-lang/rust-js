# 0184. `ok_or_else` keeps its value, a branch's operand is dropped in its branch, and `by_ref()` of the crate's iterator

Status: Accepted. Extends [0098](0098-destructors.md),
[0071](0071-stepping-iterators.md) and [0179](0179-option-filter-map-or-drops.md).

## Context

bitflags stopped at three gaps, each of a kind other crates have too. Its
parser reads a flag's name in a branch:

```rust
let parsed_flag = if let Some(flag) = flag.strip_prefix("0x") {
    ..
} else {
    B::from_name(flag).ok_or_else(|| ParseError::invalid_named_flag(flag))?
};
```

- **`ok_or_else` of a value with a destructor was an error,** a `B: Flags`
  being one that may have one, as a std call that takes such a value might
  drop it where JS wouldn't.
- **A temporary with a destructor in a branch was an error:** a
  temporary's `finally` goes around the rest of its scope, the statement,
  and its `const` was in the `else`.
- **`for (n, f) in self.inner.by_ref()`, of an iterator of the crate's,
  was an error:** `by_ref()` was std's iterators' only.

## Decision

- **`ok_or_else` keeps its value:** a `Some`'s moves into the `Ok`, and a
  `None` has none. Its function runs only for a `None`, so one that holds a
  value with a destructor, which Rust would drop unrun, is still an error.
- **An operand temporary in a branch is dropped by a `finally` in its
  branch:** an operand is moved where it's made, so by the branch's end its
  flag is false, and its `finally` only drops it if what comes before the
  move panics or leaves. A temporary of a place, which Rust drops at its
  scope's end, after the branch, is still an error.
- **`by_ref()` of an iterator of the crate's is the iterator itself,** as
  std's is: `for x in it.by_ref()` is `for x in &mut it`. One whose impl
  writes its own `by_ref` is called as written: rustc resolves the call to
  it first.

## Why

- **Each is exact:** the `option_combinator_drops` corpus case compares
  `ok_or_else` of a `Some` and a `None`, at a statement and in either branch
  of bitflags' shape, two in one branch too, with native Rust; and
  `iter_by_ref`, bitflags' `IterEqualNames`.
- **bitflags compiles,** with its whole graph.

## Alternatives

- **Declare a branch's temporary before the statement,** `let t;`, and drop
  it at the statement's end: a place's would drop at the right time too,
  but each temporary takes a flag of its own, set in its branch, to say
  it was made.

## Costs

- **A temporary of a place in a branch is still an error,** as in an
  edition-2021 block's tail: `let e = { d.incr().incr() };`.
- **A crate that uses bitflags' macro doesn't compile yet:** the macro
  implements bitflags' `Flags`, whose default methods are bitflags', which
  a consumer can't copy (ADR 0100), and calls `fmt::LowerHex::fmt` of the
  bits. (Amended: it does, and runs as natively, ADR 0185.)
