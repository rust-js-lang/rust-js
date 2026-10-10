# 0314. Each std data structure works or is refused, and coverage only grows

Status: Accepted. Organizes ADRs 0025, 0059, 0068, 0121 and the rest of
std's types.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

JS has a few data structures, Rust's std many. rust-js gives each Rust one
the JS one that behaves the same, `Vec` an `Array`, `HashMap` a `Map`,
`String` a string; a runtime helper where none does, `BinaryHeap`'s
`$heapPush`; or a compile error. But nothing said how much of each is
covered: which of `Vec`'s methods compile, which don't, and whether a
change lost one. What one type is, its JS, its methods, its `Clone` and
`Debug`, is spread over a dozen modules, and `LinkedList`, `Weak` and the
once and lazy cells weren't anywhere. Even `Vec::with_capacity` was refused.

## Decision

**Every std data structure, and each of its methods, either compiles with
Rust's meaning or is refused with a clear compile error, and how many of
each compile is measured against std itself and only grows.**

- **The registry**: `recognition/registry.rs` lists std's data structures,
  each by its name and where std defines it: `Vec` to `Cow`, `OnceCell`
  and `LazyLock`, `Box`, and `str`, slices, arrays and `char`. (Amended by
  ADR 0329: `Pin` too.)
- **The measure**: `rust-js --std-coverage <file>` writes each stable
  inherent method of each, `+ Vec::push` where `classify` knows it, `-
  Vec::shrink_to` where it doesn't, under `# Vec 18 of 48`.
- **The ratchet**: `docs/std-coverage.txt` is that list, which
  `test/std-coverage.test.ts` holds: a method known before and refused now
  fails, and one newly known is blessed in (`BLESS=1`), so the diff shows
  it, as webapi's coverage of the DOM is held (ADR 0102).
- **One type, one home**, next: each type's representation, methods and
  traits move to a module of its own, `std/vec.rs`, `std/btree_map.rs`,
  `std/rc.rs`, from where they're spread today, one type a commit, its
  output unchanged; then the types with none get theirs.

## Why

- **It's a promise that can be checked**: "all of std" can't be done at
  once, and some of it can't be kept in JS at all (`Weak` needs a count
  of strong references JS's collector doesn't keep). "Works or is
  refused, and the count only grows" can be checked on every change.
- **It's std's own list**: the methods are rustc's, so a new std release's
  are counted where they appear.
- **It's tested**: the ratchet runs in the suite, as the architecture test
  holds the registry to recognition, the only module that spells std's
  names.

## Consequences

- `classify` knowing a method is the measure: one it knows only for some
  types of items, `contains` of items compared by value, counts as known.
- Today: `Vec` 18 of 48, `VecDeque` 28 of 55, `HashMap` 15 of 33,
  `BTreeMap` 14 of 31, `String` 17 of 43, `str` 24 of 83, slices 32 of 133,
  `Option` 27 of 46, `Result` 19 of 36; `LinkedList`, `Weak`, `OnceCell`,
  `LazyCell`, `OnceLock`, `LazyLock` and `Cow` none.

## Amendment: a method known for some of its own type arguments

A method generic over its own type arguments, a `str`'s `split<P:
Pattern>` or a slice's `get<I: SliceIndex>`, is known only for some, a
`&str` or a `char` pattern, an index; classified with them left as
parameters it was counted unknown, though it compiles. The measure now
tries each of its own type parameters as `&str`, `char` and `usize`, and
counts it known if `classify` knows it for one: `str` 45 of 83, not 24,
and slices 35 of 133, not 32. The rule is the Consequences' first: known
for some types counts as known.
