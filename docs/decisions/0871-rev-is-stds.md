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

## Amendment: a `Rev` a recognizer gives to be held is std's too

memchr's `memrchr_iter`, which its crate writes as `memchr_iter(..).rev()`,
was `$rev($memchrIter(..))` (ADR 0529), a `Rev` given back to be held: its
`clone()`, std's compiled `Clone`, read the struct's `iter`, which `$rev`
hasn't, and threw. **A `Rev` a recognizer makes to be held is std's struct,
`iterator_rev($memchrIter(..), { next: $next }, { next_back: $nextBack })`**
(`FnCx::held_rev`), as every held `Rev` is. And `clone_value`'s own clone
of a `Rev`, `$rev` of a clone of its `source`, is gone: a `Rev`'s `Clone` is
its compiled impl's, which `clone_value` asks first. Tested by
`shipped-crates.test.ts`, memchr's searches against native memchr's.

## Amendment: `Enumerate` is std's where it's held too, and a variable that only steps one is in place

P1's second adapter. **`Enumerate` is std's struct where it's held,
`{ iter, count }`, compiled as `Rev` is**: `Iterator::enumerate`, `next`,
`size_hint`, `nth`, `count`, `fold`, `next_back`, `nth_back`, `rfold`,
`len`, its clone and its `Debug`; `try_fold` and `try_rfold` wait as
`Rev`'s do. It's no explicit adapter any more (ADR 0542): a kind std's
struct is where it's held is its own kind of held, so `explicit_adapters`
leaves out `HELD_COMPILED`'s, the one catalog of them, `Rev`'s and
`Enumerate`'s, which `makes_held_compiled` and `is_held_compiled` answer
of.

- **In place is a fact of the whole chain**: an adapter given to another
  of them that's made in place is in place too, `enumerate().rev()`'s
  first, found by `in_place_adapters` to its fixed point.
- **A variable that only steps one is in place**: a `let` of a `rev()` or
  an `enumerate()` read only where `Iterator`'s methods step it as a JS
  iterator, `next()`, a `for` loop, a `by_ref()` whose `&mut` they step,
  or `mem::drop`, which closes either, shows nothing of what it is, so
  it's the recognizers': `let mut cs = s.chars().enumerate()` is
  `Array.from(s).entries()` again, and a reversed range held so, its
  `$iterator(range, $rangeNextBack)`.
- **What the recognizers made, a step past the call that made it can
  tell**: `iterators::made_in_place`, `$rev(..)`, `$enumerate(..)`, `xs.map((x,
  i) => [i, x])` or an array reversed, is never given std's compiled back
  by `knowing_held`, which `rev()` of an in-place `enumerate()`'s `map`
  asks.
- **A held compiled iterator stepped from its back by a `rev()` asks what
  its compiled `next_back`, `nth_back` and `len` step**, by their
  manifests, of the type parameters it holds, `I`'s length of a held
  `Enumerate<I>`; not what its impl's bounds ask, which would give each
  function dictionaries it never reads.
- **A struct lent by `by_ref()` is stepped where it is**, as std's `&mut
  I` steps its `I`: `$lent` of std's compiled struct reads and writes its
  fields, `iter` and `count`, through to it, its own steps first, so a
  `$Steps` helper's name, `count`, never hides one; what stops early
  closes the lent, not it. And the `ExactSizeIterator` dictionary of a
  `&mut` to the crate's iterator is the iterator's.

Tested by `std_enumerate.rs` against native Rust; the corpus's 36 programs
that enumerate and 58 that reverse, as apps and libraries. The readability
baseline is blessed with it: a held `Enumerate` of the crate's iterator is
std's struct, whose `next_back` and `rfold` are its compiled calls,
`while (true)` 10 to 10.1, `return;` 9.4 to 9.6 and renames 720.9 to 721.2
in ten thousand lines, of `std_enumerate.rs` itself mostly.

## Amendment: `Map` is std's where it's held too

P1's third adapter. **`Map` is std's struct where it's held, `{ iter, f }`,
compiled as `Rev` and `Enumerate` are**: `Iterator::map`, `next`,
`size_hint`, `fold`, `next_back`, `rfold`, `len`, its clone and its
`Debug`; `try_fold` and `try_rfold` wait as theirs do. `HELD_COMPILED`
gains it, and so does what's one of std's iterator adapters
(`is_iter_adapter_def`): a held-compiled kind is an adapter where it's made
in place, so a `map()` a `rev()` steps from its back is made knowing it,
`$mapEnds`, as before.

- **An iterator trait's method on one made in place is the recognizers'**:
  `v.iter().map(f).rfold(..)` and `.len()`, `DoubleEndedIterator`'s and
  `ExactSizeIterator`'s, step the JS iterator the recognizers made, not
  std's compiled impl (`steps_adapter_in_place`); `clone()` and `fmt`, any
  other trait's, are its struct's. `len()` asks `compiled_struct_at`, the
  value's form, not the type's, as every such question now does.
- **A variable read only by another made in place of it is in place**:
  `let cs = text.chars().enumerate(); cs.map(f)` is `entries()` still, to
  the fixed point; one read by another that's held is held, `let mut back
  = cubes.rev()`'s `cubes`, std's struct the held `Rev` steps.
- **What a closure gives `flat_map` is made in place**: `flat_map(|g|
  g.iter().map(f))` steps the closure's `map()` where it's given, so it's
  `g.map(f)`, as one given to a std adapter is (`results_in_place`, of the
  crate's bodies before they're lowered).
- **Cloned where it's held, by its compiled `Clone`; cloned apart where
  it's made in place** (ADR 0525): a `Map` that's part of a `filter` or a
  `chain` cloned apart, or what `cycle()` takes, is the JS iterator the
  recognizers made, which its clone makes again (`clone_in_place`); one a
  struct holds, std's `mapClone_clone`. `cycle()` asks which by
  `compiled_struct_at`.
- **Generic code given std's struct is given its compiled steps**: the
  `DoubleEndedIterator` and `ExactSizeIterator` dictionaries of a compiled
  struct, a held `Map`'s or `Rev`'s, are its compiled `next_back`, `rfold`,
  `nth_back` and `len`, not a JS iterator's `$nextBack` it hasn't. Its
  `rfold` is given no drops of what it folds into and by, as the crate's
  own is (#162): its own `B` and `F` are `()` and a function of it
  (`compiled_rfold`). A held `Rev` an enum holds, refused before, is
  stepped from its back so.
- **A closure std's MIR inlines holds its maker's caller's types**: rustc
  inlines `map_fold` into `Map::rfold`, so its closure's captures are `F`
  and `G` of the `rfold`, read from the closure's own type as the body
  makes it, not from `map_fold`'s; its carried drop dropped `F`'s capture
  by `Acc`'s drop before.
- `held_hint_refused.rs` runs now: a held `Map`'s `len()` and `size_hint()`
  are its compiled ones. `held_filter_map_back_refused.rs` is still
  refused, by the dictionary a held `Map` asks of its `FilterMap`.

Found on the way, older and filed: a closure a function makes, given to
`fold`, is made again for each item and never dropped (#259); a
`flatten()` of an `Option`'s iterator stepped from its back throws
(#260), which `Flatten`, next, takes.

Tested by `std_map.rs` against native Rust, its drops of what an `rfold`'s
closures took too; the corpus's programs that map, as apps and libraries.
The readability baseline is blessed with it: `while (true)` 10.3 to 10.2,
`$iter` 75.4 to 76.2, `return;` 9.7 to 9.8, of `std_map.rs`'s own early
returns, and renames 723.5 to 720.9 in ten thousand lines.
Map's coverage 3 of 8 to 6 of 8.
