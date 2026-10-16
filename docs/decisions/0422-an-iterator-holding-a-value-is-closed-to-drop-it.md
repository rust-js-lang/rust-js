# 0422. An iterator that holds a value is closed to drop it

Status: Accepted. Extends [0098](0098-destructors.md) and
[0421](0421-std-associated-type-drops-are-given.md), and amends
[0190](0190-generic-no-destructor.md).

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
