# 0468. A lent adapter leaves its source to its lender, lent once

Status: Accepted. Amends [0422](0422-an-iterator-holding-a-value-is-closed-to-drop-it.md); extends
[0465](0465-dropping-an-adapter-is-closing-it.md) and
[0071](0071-stepping-iterators.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, and
when, as std's.

## Context

itertools takes the first `k` of an iterator it goes on stepping,
`iter.by_ref().take(k).collect()`, of items its caller may give a
destructor, in `k_smallest`, `tail` and `max_set`. That was refused (ADR
0422): JS's `take` closes what it steps as it ends, and the lender still has
the rest. But what `take` steps is `$lent(iter)`, which steps `iter` and
can't close it (ADR 0071). Rust's `Take<&mut I>` drops nothing of `I`, and
pulls no more than it gives.

And a lent iterator was lent again: `by_ref()` lent it, then the adapter's
`&mut` receiver lent that, `$lent($lent(it)).take(2)`, and generic code
made the lent one a JS iterator again, `$lent(Iterator.from($lent(..)))`.

## Decision

**An adapter of a lent iterator, `by_ref().take(n)`, leaves what it hasn't
given to its lender**, which drops it as it drops the rest: the source is
`$closing` only when it's moved into the adapter (ADR 0465), and a lent one
is itself, whichever adapter holds it.

**An iterator is lent once**: lending `$lent(it)` is itself, and so is
making it a JS iterator, which it already is.

```js
first = $lent(Iterator.from(iter)).take(k).toArray();
const evens = $lent(nums).filter((n) => n % 2 === 0).take(2);
```

## Why

- **It's std's**: `take_destructors` steps a lent `take` of a `Vec`'s
  items, and of a generic iterator, then the rest, and drops each when
  native Rust does.
- **One rule**: whether an adapter closes its source is whether it owns
  it, the same for `take` as for `map` and `filter`.
- **It reads as hand-written JS**: one `$lent`, where the source is lent.

## Consequences

- **What generic code may do grows**: `by_ref().take(k)` of items with a
  destructor, as itertools' `k_smallest`, `tail` and `max_set` do.

## Amendment (2026-11-12, #1508): what's lent is never `Iterator.from` of it

Generic code that lends what it's given, `it.by_ref()` or `&mut it`, was
`$lent(Iterator.from(it))`. `Iterator.from` calls its argument's
`[Symbol.iterator]()`. A `$Steps`' arms it to close itself at its end,
as a JS consumer that takes it by value would. So the lender, not the
lent view, closed at its first `None`: a given iterator with a `Drop` was
dropped there, stepped on after its drop, and not dropped where Rust
drops it. Now `emit`, `$lent`'s one writer, lends `x` of an
`Iterator.from(x)`, `$lent(it)`. `$lent` makes a JS iterator of what has
no `next` itself, an array, as `Iterator.from` did. Results change, so
Rust's behaviour is kept (ADR 0262's third row). The coordinator approved
the form. It removes a call, and 20 snapshots lose their
`Iterator.from`.

Proof: the corpus case `lent_given_iterator_dropped_where_it_goes`,
against native Rust in both builds. Generic code steps what it's given by
`next` twice past a `None`, by `by_ref().sum()`, by a `for` over
`by_ref()` and over `&mut it`, and `by_ref().collect()`s it past a
`skip(3)`. The metamorphic net's `loud` rewrite of `restarting_iterators`
found it.
