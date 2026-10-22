# 0429. A generic or `dyn` double-ended iterator is a JS iterator that knows its back

Status: Accepted. Amends [0061](0061-generic-iterators.md) and
[0140](0140-iterator-trait-objects.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): each step from the
back is the one Rust's takes, of the iterator itself, so a range lent as
one is moved where Rust's is. Where it can't be, D: rust-js's error.

## Context

A generic iterator is a JS iterator (ADR 0061), and so is a `dyn
Iterator` (ADR 0140). A JS iterator only steps from the front, so
`next_back()` and `rev()` of a `T: DoubleEndedIterator`, or of a `dyn
DoubleEndedIterator`, were errors:

```rust
fn pairwise_sub<T: DoubleEndedIterator<Item = u32>>(mut t: T) -> u32 {
    let mut result = 0;
    while let (Some(front), Some(back)) = (t.next(), t.next_back()) {
        result += back - front;
    }
    result
}
```

The value given for `t` steps from its back already, every one: an
array's `$iter` has its end (ADR 0071), a range its `end`, and one of the
crate's its `next_back`. It was just made a JS iterator that forgot it.

## Decision

**A type parameter bound by `DoubleEndedIterator`, and a `dyn
DoubleEndedIterator`, is a JS iterator that knows its back, `back()`,
which `$nextBack` steps.** Its callers make one where they give it:

| Given | JS |
|---|---|
| An array's iterator | its `$iter`, which knows its end |
| A range | `$iterator(range, $rangeNext, false, $rangeNextBack)` |
| One of the crate's own | `$iterator(it, spanIterator_next, false, spanDoubleEndedIterator_next_back)` |
| One already, a parameter's or a `dyn`'s | itself |

```js
function pairwise_sub(t) {
  let result = 0;
  while (true) {
    const front = $next(t);
    const back = $nextBack(t);
    if (front == null || back == null) {
      break;
    }
    result = (result + ((back - front) >>> 0)) >>> 0;
  }
  return result;
}
pairwise_sub($iterator({ start: 0, end: 10 }, $rangeNext, false, $rangeNextBack));
```

- **`$iterator`, the one JS form of a Rust iterator stepped by its
  functions, takes its `next_back` too**, after whether its `Some` is
  boxed, and gives it `back()`. A `$iter` has its own. `$nextBack` and
  `$nextBackSome` step either, as `$next` and `$nextSome` step the front.
- **`rev()` of one is `$iterator(t, $nextBack)`**, its back first, as a
  crate's own reversed is `$iterator(it, next_back)` (ADR 0164).
- **One question, `knows_its_back`,** says which types these are, of what
  generic code steps and of what its callers give it, and
  `as_double_ended` makes one of a value, of a call's argument and of a
  value made a `dyn` alike.
- **Not one given as it is** (ADR 0411), a parameter a type holds that's
  bound by `Iterator` too: it's stepped by its `Iterator` dictionary, which
  has no `next_back`, so that is still refused, as is a value that can't be
  stepped from its back as it is, a lazy chain's.

## Why

- **It's the program Rust runs:** the `generic_double_ended` corpus case
  steps an array's, a range's and the crate's iterator from both ends,
  generic and as a `dyn`, boxed and lent, reverses them, passes one on, and
  steps items that look like `None`, with native Rust. A range lent as a
  `&mut dyn DoubleEndedIterator` is moved, as Rust's is.
- rustc's `associated-types-iterator-binding.rs` and
  `associated-types-doubleendediterator-object.rs` pass.
- **It's what a person writes:** the iterator they have, and `next` and
  `back` of it; no dictionary to carry.

## Alternatives

- **A `{ next, next_back }` dictionary, as ADR 0411 gives one held as it
  is.** Every caller would pass one, where the value can carry its own
  `back()`, as it carries `next()`.
- **An array of what's left.** A range's every item would be made at once,
  and a lent one wouldn't be moved.

## Consequences

- `$nextBack(it)` calls `it.back()` where it read `it.items` and `it.end`
  itself; a `$iter` steps from its end as it did.
- `rev()` of a reversed one, `t.rev().next_back()`, is still an error.

## Amendment: `rev()` of a `dyn`, held by a case

`test/corpus/rev_of_dyn_double_ended.rs` holds `rev()` of a `Box<dyn
DoubleEndedIterator>`, given away and held, after a `next_back()`, as
`rev_items` makes it, `$rev(it)`: the nightly found the other place that
made it, in `reversed`, never decided it. The readability measure's
`$iter` share grows, 62.4 to 62.5 in ten thousand lines, from the vectors
it and `held_enumerate`'s widened case give as iterators, `$iter([1, 2,
3])`.
