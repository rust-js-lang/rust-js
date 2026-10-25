# 0871. `Rev` is std's where it's held; the recognizers reverse what's stepped where it's made

Status: Accepted. Extends [0561](0561-std-bodies.md) and
[0566](0566-fuse-is-stds.md), whose `Fuse` it follows; amends
[0516](0516-rev-steps-what-it-holds-from-its-back.md), whose `$rev` is the
recognizers' form of a `Rev` stepped where it's made,
[0545](0545-a-map-stepped-from-its-back-is-made-knowing-it.md), whose
analysis makes the source of a held `rev()` knowing its back, and
[0526](0526-a-filter-steps-from-its-back.md), whose `$filterEnds` takes
items with a destructor too.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what a held `Rev`
steps, from either end, what it counts, drops, clones and shows is std's,
as its bodies say. N for the JS of one stepped where it's made, which the
recognizers write as before.

## Context

The review of the architecture (P1) asked that std's adapters be compiled
from the MIR the toolchain ships, as `Peekable` and `Fuse` are, not modeled
by hand. `rev()` was the hand model's: `$rev(it)`, which steps `it` by its
`back`, and forwards its `fold`, `nth` and `try_fold` to `it`'s `rfold`,
`nth_back` and `try_rfold` by hand (#220); a reversed range held, a
`$iterator(range, $rangeNextBack)` that `nth` read the bounds of, `.of`; a
held one's `Debug`, clone, `len` and `next_back` each its own rule, and
`next_back()` of a held `Rev` of the crate's iterator or of a generic one
refused: `rust-js does not support next_back() of a lazy or generic Rev`.
std's `Rev<I>` is `{ iter: I }`, twelve bodies, each `safe` by
`--std-coverage`.

The user chose (2026-10-25) Fuse's way over compiling it only for the
crate's own and generic sources: one representation of a held `Rev`, the
recognizers keeping their output where a chain is consumed in place.

## Decision

**`Rev` is std's struct, compiled into `@rust-js/std`: `Iterator::rev`,
`next`, `size_hint`, `nth`, `find`, `fold`, `next_back`, `nth_back`,
`rfold`, `rfind`, `len`, its clone and its `Debug`. A `rev()` held, by a
`let`, a field, a return or an argument of its own type or one given as it
is (ADR 0411), is that struct, `iterator_rev(it, IIterator,
IDoubleEndedIterator)`; a `rev()` stepped where it's made, as a JS
iterator, is what the recognizers make of it, `v.toReversed()`, `$rev(it)`.**
`try_fold` and `try_rfold` stay the hand model's: rust-js doesn't lower a
generic `try_fold` into an `R: Try`, which std's body is.

- **Which `rev()` is stepped where it's made is a fact of the body's THIR,
  `body_queries::in_place_revs`**, the one answer: a `rev()` whose value
  is given, as the receiver or an argument, borrowed for it or not, to a
  std function or method not compiled, a `map`, a `for` loop's
  `into_iter`, `zip`, `String::from_iter`; or to a type parameter of the
  crate's function given a JS iterator (ADR 0061). Not a trait's method,
  but `Iterator`'s and `IntoIterator`'s, called on the `Rev` itself,
  `next_back()`, `len()`, `clone()`, `Rev`'s own compiled impl's. It
  gives the `rev()`'s `fn_span` and that call's, which MIR's calls carry.
  The dispatch takes such a `rev()` to the recognizers (`rev_in_place`);
  `compiled_struct_at` answers, for the call that steps it, that its value
  isn't the struct, where `dropped_after`, `closing_drop` and
  `knowing_held` ask.
- **A held `Rev`'s source is made knowing its back where it's made**: the
  compiled `next` steps it by its `DoubleEndedIterator` dictionary, so
  `back_stepped` (ADR 0545) counts the type a held `rev()` asks, of an
  array's items too, and a std adapter given to it as it is is made
  double-ended as a function's `I: DoubleEndedIterator` is. A filter of
  items with a destructor is `$filterEnds` of a test that drops what it
  doesn't keep, as the forward filter's is (ADR 0098).
- **What a compiled std struct holds, its impls' `size_hint` steps too**:
  `iterator_steps` asks a struct a body makes for its `next` and its
  `size_hint`, so `rev()`'s `Self` is stepped by `Iterator`, which its
  hint asks, as well as `DoubleEndedIterator`, which its `next` asks.
- **A compiled struct of a kind dropped by closing, `Rev`'s, closes what it
  holds**: `$stdIterator`'s `steps.close` drops each field by its drop,
  once, as Rust drops the struct; a chain over it closes it as it's closed,
  as it closed `$rev`. `Peekable`'s and `Fuse`'s, dropped by their fields'
  drops, close nothing, as before.
- **A range's `DoubleEndedIterator` dictionary has its own `nth_back`**,
  `$rangeNthBack`, of its bounds, so a held reversed range's `nth`, std's
  `iter.nth_back(n)`, never steps each number (#163); a held one's bounds
  for `range_own_step` are its `iter`, one made in place its `$iterator`'s
  `of`, as the same fact says.

```js
// `let mut r = v.iter().rev(); r.next(); r.next_back()`
const r = iterator_rev($iter(v), { next: $next }, { next_back: $nextBack });
$next(r);
revDoubleEndedIterator_nextBack(r, { next: $next });
// `v.iter().rev().map(|x| x + 1).collect()`, stepped where it's made
v.toReversed().map((x) => (x + 1) >>> 0);
```

## Why

- **It's std's**: the 58 corpus programs that reverse, native Rust their
  oracle, print what they printed, as apps and as libraries; a held `Rev`
  of the crate's iterator's `next_back()`, refused before, is std's
  `next_back`, its source's `next` (`test/corpus/std_rev.rs`).
- **One representation of a held `Rev`**: a struct, wherever it's held or
  given, so generic code given one, or returning `Rev<I::IntoIter>`, reads
  what concrete code made. One stepped where it's made is never held, so
  what the recognizers make of it, an array reversed or a `$rev`, can't
  reach code that reads a struct.
- **Rust's facts, not the JS's shape**: which `rev()` is held is read from
  THIR, as the review's P2 asks of what passes decide, not from the JS a
  temporary holds when its consumer is lowered.

## Consequences

- **The app's JS**: a held `Rev` is `iterator_rev(it, { next: $next },
  { next_back: $nextBack })` where it was `$rev(it)`, and its
  `next_back()`, `len()`, clone and `Debug` the library's calls; a function
  returning `Rev<I::IntoIter>` takes `I`'s `IntoIterator` dictionary, as
  one returning a `Fuse` does (ADR 0566). One stepped in place reads as it
  read. The readability baseline is blessed with them: `$iter` 72.8 to
  73.4 in ten thousand lines, a held `Rev`'s source, `iterator_rev($iter(v),
  ..)`, where `$rev` closed it, and `return;` 9.4 to 9.5.
- `$rev` stays, the recognizers' form of one stepped in place, with its
  forwarding of `fold`, `nth` and `try_fold`; a held one's are std's.
- Next, as the review lists: `Enumerate`, `Map`, `Flatten` and `Inspect`,
  each its own decision of this shape, and a generic `try_fold` into an
  `R: Try`, which `Rev`'s `try_fold` and `try_rfold` wait for.
