# A library's manifest says which `Rc`s it counts

Status: Draft. Would extend [0320](../0320-counted-rcs.md), whose counted
`Rc` in a library is refused, and [0494](../0494-a-generic-rc-is-counted-where-it-may-drop.md),
whose generic `Rc` counts every `Rc` of a binary; and
[0414](../0414-a-library-on-npm-as-rescripts-are.md), whose manifest lists
a library's drops.

Case: C ([0262](../0262-when-rust-and-js-disagree.md)): each count is
Rust's at each point, and the last `Rc` drops what it points at, once, in
every crate that holds it.

## Context

An `Rc<T>` is counted, `{ value, strong, weak }`, where its counts are
read or its value's drop runs; every other is its value (ADR 0320). A
counted `Rc` in a library is refused: a consumer would hold the same `Rc`
as the value, and the two would disagree on what one is. ADR 0494 made a
generic `Rc` of a type parameter's value count every `Rc`, and left a
library's, itertools' `tee` of an `Rc<RefCell<TeeBuffer>>`, refused: the
one thing itertools still refuses.

The question "is this `Rc` counted" has one owner inside a crate,
`Counted`. Across the crate boundary it has none: the library knows what
it counts, and the consumer works it out again from its own bodies, which
never see the library's. The rule for a boundary is that the owner reports
and the other side reads (AGENTS.md), as a library's manifest already
reports each item's drops.

## Decision

**A library's manifest reports which `Rc`s it counts, `counted`: the
pointee types whose `Rc` is `{ value, strong, weak }`, by path, and
`counts_every_rc` where a generic `Rc` makes every one counted (ADR 0494).
A consumer's `Counted` starts from every library it uses, and reaches its
fixed point from there: an `Rc` a library counts is counted in every crate
that holds it, and one a library holds as a value is never counted by a
consumer.**

```json
{ "name": "itertools", "counted": [], "counts_every_rc": true }
{ "name": "tree", "counted": ["tree::Node"], "counts_every_rc": false }
```

- **One owner each way**: the library's `Counted` answers for its own
  bodies and fields and writes the manifest; the consumer's reads every
  dependency's before it looks at its own bodies, in `Dependencies`,
  beside each library's `Exports`. Nothing re-derives a library's answer
  from its types.
- **A consumer that must count what a library doesn't is refused, loud**:
  `Rc::strong_count` of an `Rc<Foo>` a library makes and holds as its
  value, naming the library and the type, as ADR 0320 refuses it today.
  The fix is the library's: read a count there, or hold a counted type.
  (Later, a library could count every `Rc` on request, a `Cargo.toml`
  key; not now, one way first.)
- **`counts_every_rc` is contagious**: a consumer of a library that counts
  every `Rc` counts every `Rc`, as a binary with a generic `Rc` does (ADR
  0494); a library that holds no `Rc` at all reports neither, and costs
  its consumers nothing.
- **Two libraries that disagree**, one counting `Rc<Foo>` and one holding
  it as a value, are refused where the app uses both, naming each: no
  representation serves both.
- **The manifest's version rises**, `library::VERSION`, so a library built
  before this is rebuilt, never read with no `counted`.

## Why

- **It's std's**: the crates test shares a counted `Rc` between a library
  and its consumer, cloned in one and dropped in the other, with
  `strong_count` read on both sides, and a `Drop` type dropped by the last
  clone wherever it is, with native Rust; and itertools' `tee` runs as a
  library, two iterators over one buffer, dropped by the last.
- **It's the boundary rule**: the owner reports, the reader reads, and a
  test holds them equal, as a library's drops are today.

## Alternatives

- **Count every `Rc` everywhere**: no analysis, no manifest field, and
  every `Rc` nobody counts `{ value, strong, weak }` in a `try`. Against
  the output promise and ADR 0320's choice; it would also change every
  snapshot.
- **Count every `Rc` of any library**, coarser than the manifest: a flag
  with no types, so a library holding one plain `Rc` would count its
  consumers' every `Rc`. A step that becomes this draft's `counts_every_rc`
  alone, so start with the whole.
- **Refuse, as now**: itertools' `tee` stays the one thing it refuses, and
  every library that holds an `Rc` of a `Drop` type is refused.

## Consequences

- **What compiles grows**: a library's counted `Rc`, a library's `Rc` of a
  type with a destructor, itertools whole.
- **A library's consumers may count more `Rc`s than their own code asks**,
  where the library does: the price of agreement, paid in the output
  ADR 0320 already prices.
- **`docs/crate-corpus.md`**: itertools runs.
