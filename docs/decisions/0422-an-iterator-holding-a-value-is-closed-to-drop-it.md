# 0422. An iterator that holds a value is closed to drop it

Status: Accepted. Extends [0098](0098-destructors.md) and
[0421](0421-std-associated-type-drops-are-given.md), and amends
[0190](0190-generic-no-destructor.md). Extended by
[0465](0465-dropping-an-adapter-is-closing-it.md): every std lazy adapter
is closed to drop, and `$closing` closes by its source's own drop. Amended
by [0468](0468-a-lent-adapter-leaves-its-source-to-its-lender.md): a `take`
of a lent iterator is no longer refused, what it hasn't given its lender's.

Case: C ([0262](0262-when-rust-and-js-disagree.md)) for what's dropped and
how many times; B for when, where a `take` is stepped past its end before
it's dropped, listed below.

## Context

`iter::repeat(value)` holds `value`, gives a clone of it each step, and
drops it when it's dropped. smallvec's `resize` fills with it:

```rust
self.extend(repeat(value).take(len - old_len));
```

`$repeating(value, clone)` was a generator, which holds `value` where
nothing can reach it to drop it, so `repeat` of a value with a destructor
was refused, and so was `take` of its items, which may leave some behind.
JS has its own way to end an iterator early: `return()`, which closes it,
and which JS's `take` and `drop` helpers pass on to what they step.

## Decision

**An iterator of std's that holds a value, `repeat`'s, drops it when it's
closed, JS's `return()`, and dropping it, or a `take` or `skip` of it, is
closing it: `it.return()`.**

```js
const it = Iterator.from($repeating(["b"], undefined, loudDrop_drop)).take(3);
const first = $next(it);
it.return();
```

- **Which iterators those are is `recognition::closed_on_drop`'s**, and
  what each holds: `repeat`'s value, and a `take`'s or a `skip`'s of one,
  JS's helpers, which close what they step.
- **`take` of one leaves nothing behind**: it makes each item as it's
  stepped, so `take` and `skip` of items with a destructor are its; and
  generic code that hands a `repeat` of its `T` to `take` takes a
  destructor of `T`, the `take`'s drop being the value's.
- **`v.extend(items)` of items with a destructor** moves each into `v`;
  what it drops of `items` once it's given them all is nothing, of an
  array, a `Vec`, an `Option`, their iterators, or one closed as it ends
  (`values::ends_holding_nothing`). An iterator of the crate's, whose
  `Drop` runs then, is refused still.

And generic code's std calls that drop none of what they're given, which
ADR 0190 took as dropping it, so a caller's `T` with a destructor was
refused: `swap`, a slice's `get_mut` and `get_unchecked_mut`, a generic
index's `index_mut`, `extend_from_slice`, and the allocator's, `set_len`
(ADRs 0417, 0420).

## Why

- **It's the program Rust runs**: `test/corpus/repeat_drops.rs` fills a
  vector with `repeat(value).take(n)`, in generic code too, drops one
  stepped once, one never stepped, one looped over and a `skip` of one,
  and extends with each kind of iterable, printing each drop where native
  Rust does; `test/corpus/generic_drops_nothing.rs` swaps, lends, extends
  and shortens a generic `T`'s vector given one with a destructor.
- **It's JS's own disposal**: `return()` is what a `for` loop calls when
  it leaves early, and what `take` calls when it's done.

## Consequences

- **A `take` closes what it steps when it's stepped past its end**, as
  JS's does, so `repeat`'s value is dropped then, not when the `take` is:
  a program that steps one past its end and does something it can see
  before dropping it sees the drop first. A loop, and `extend`, drop it
  as they end, as Rust does (case B).
- `map`, `chain` and the rest of std's adapters of a `repeat` of a value
  with a destructor are refused still: rust-js's own adapters needn't
  close what they step.
- `vec![value; n]` of a value with a destructor is refused still: of
  `n == 0` it drops `value`.

## Amendment: a `take` of an iterator that owns its items

`take(n)` of an iterator that owns items with a destructor, moved into it,
`v.into_iter().take(2)`, `drain(..)`'s or `Some(x).into_iter()`'s, was
refused. Its source is `$closing(it, drop)` now, whose `return()` drops
each item it hasn't given, which JS's `take` calls as it ends, and a loop
that leaves early, or dropping the `take`, calls: what Rust drops with the
`take`, dropped where Rust does, within a loop's last step (case B, as
above). Which iterators own their items is `recognition::owned_items`, the
one question their own drop asks too. A `take` of one that's lent,
`it.by_ref().take(n)`, is still refused: it would close what its lender
still has. `skip(n)` of one is `$skipDropping(it, n, drop)`, which drops
what it passes over as it's first stepped, as Rust's `nth` does, and what
it hasn't given when it's closed, where JS's `drop` would pass over them
undropped. The `take_destructors` corpus case runs each with native Rust.

## Amendment: a side that holds nothing to drop (#406)

A zip of a slice's `iter_mut()`, a `chars()` or a map's `keys()` and a
`Vec` whose items have a destructor threw where it ended: `for (const item
of iter.b) ..`, std's `Zip`'s drop of its fields, of a `$lazyZip`, which
has no `b`. Closing such a zip drops what's left of the `Vec`, but
`closing_source` gave no answer of its other side, only an array's
`iter()` and a range being known to hold nothing. **A side that holds
nothing a drop runs code for, as rustc's `needs_drop` says of its type,
closes dropping nothing**, so the zip is closed as it's dropped,
`iter.return()`. Case R: the JS threw where Rust drops. Its case's zips
step in `while (true)`, as a closing one does, the readability baseline's
share of those blessed for it. Held by
`test/corpus/zip_of_borrowed_and_owned.rs` against native Rust, at the
zip's end and at a `break`, and the mutation
`closing-source-holding-nothing-unclosed` (`recognition/iterators.ts`).
