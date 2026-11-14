# 0140. An iterator trait object is a JS iterator

Status: Accepted. Extends [0055](0055-iterator.md), [0061](0061-generic-iterators.md), [0071](0071-stepping-iterators.md) and [0139](0139-lazy-chains.md).
Amended by [0429](0429-a-double-ended-iterator-knows-its-back.md).

Case: A, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A function that returns one of several iterators, or takes any iterator
without being generic, uses a trait object:

```rust
fn numbers(desc: bool) -> Box<dyn Iterator<Item = u32>> {
    if desc { Box::new(Countdown(3)) } else { Box::new(vec![1, 2, 3].into_iter()) }
}
fn first_two(it: &mut dyn Iterator<Item = u32>) -> (Option<u32>, Option<u32>) {
    (it.next(), it.next())
}
```

A `dyn` of the crate's own trait is a value and its dictionary (ADR 0049).
An iterator can be in JS what it is in Rust, though: something with a
`next`, whatever made it. Its items can come from an array, a lazy chain,
a `$iter` stepped through (ADR 0071) or one of the crate's own iterators
(ADR 0055), so they were an error.

## Decision

**A `dyn Iterator`, boxed or lent, is a JS iterator with JS's iterator
helpers**, made where the value becomes one:

| Given as a `dyn Iterator` | JS |
|---|---|
| A lazy chain (ADR 0139), another `dyn Iterator` | itself |
| One of the crate's own | `$iterator(value, countdownIterator_next)`, as generic code is given it (ADR 0061) |
| A generic one, an array or an iterator | `Iterator.from(items)` (ADR 0061) |
| Anything else: an array, a stepped local's `$iter` | `Iterator.from(value)` |

- **What's done with one is what's done with a lazy iterator:** `next()`
  is `$next(it)`, wherever it's kept, a parameter included; a `for` is
  `for (const n of it)`; adapters and consumers are its helpers,
  `it.map(f).toArray()`.
- **Its chain runs item by item,** as what it's given to takes them: a
  chain boxed as one is lazy from its first stage that does what can be
  seen, as a chain stepped through is (ADR 0139).
- **A local lent as a `&mut dyn Iterator` is stepped through,** by
  whatever it's lent to, so it's a `$iter` (ADR 0071), and `Iterator.from`
  of it shares its place: `first_two(&mut it)` then `total(&mut it)` sums
  what's left.
- **A `&mut dyn Iterator` is the iterator,** as a `&mut` to a JS object
  is the object.
- **A `dyn DoubleEndedIterator` is an error:** a JS iterator only steps
  from the front, so `next_back()` has nothing to call.
  (Amended by [0429](0429-a-double-ended-iterator-knows-its-back.md): it's
  a JS iterator that knows its back, `back()`, which `$nextBack` steps.)

## Why

- **It's the JS a person writes:** `function first_two(it) { return
  [$next(it), $next(it)]; }`, with no dictionary to carry.
- **JS's iterators are already dynamic.** Each kind of Rust iterator
  already becomes one where it needs to, for generic code and lazy chains;
  this makes that happen where a value becomes a `dyn`, too.

## Alternatives

- **A `{ value, impl }` pair, as for the crate's traits.** Every call
  would go through `impl.next`, and every adapter would have to be lowered
  by hand, where JS's iterator helpers already are.
- **An array or a JS iterator, as a generic iterator is (ADR 0061).**
  `next()` and a lent local's place need an iterator: an array doesn't
  know where it is.

## Consequences

- `Iterator.from` of an array is one more call than the array itself; a
  `dyn Iterator` is one only where the program asks for one.
- Of rustc's tests, 5 more pass, those that box an iterator, lend one or
  return one from a closure.

## Amendment: no rule of a generic value's type in `collected_items` (2026-11-06, #815)

`collected_items` refused to list a generic value's `Iterator.from` by its
type, `known_by_bounds`. Generic code's collect asks its room or folds it
(`$roomForHint`, `$foldToArray`, #733, #796), so no `Iterator.from` of a
generic value reaches it, and the rule changed no JS anywhere. It's gone,
with its mutation `mir-iterable-collected-generic`.
"a generic iterator collected keeps an iterator with only a next" still
holds the behaviour.

## Amendment (2026-11-13, #1476): a `dyn Iterator`'s hint is its value's, made knowing it

Case ([0262](0262-when-rust-and-js-disagree.md)): preserve. A `dyn
Iterator`'s `size_hint()` is its value's, as std's vtable gives it; it was
refused, as the JS iterator a `dyn Iterator` is knew no hint where JS's
own adapters made it.

A crate that asks a `dyn Iterator`'s `size_hint()` (`Hinted::dyn_asked`,
`analysis/hint_asked.rs`) makes each value it makes one of knowing its
hint where it's made: each coercion's source is marked as a library's
argument is (`mark_given`), a filter's `$filterEnds`, a `skip(n)`'s
`$skipping`. The call is `$sizeHint(it)`, which asks the value's own.
What's covered, each against native Rust in both builds
(`test/corpus/dyn_iterator_size_hints.rs`): a slice's items and an
adapter of them, `filter`, `skip`, `take`, `rev`, `map`, `enumerate`, a
`zip` with a range, a `chain` with a range, a `flat_map`; a range and its
`step_by`; an array's `into_iter().cycle().take(n)`; the crate's own
iterator, with its own `size_hint` and without; a `dyn` boxed again; and
a `&mut dyn Iterator` lent to a function that asks it. A crate that asks
none writes what it did.

Still refused, each naming its open issue: `size_hint()` of a `dyn
Iterator` in a crate that takes one from a library or a binding, which it
can't see made (#1562, `test/js-builtins.test.ts`); and, in a crate that
asks one's, a value made a `dyn Iterator` whose hint isn't std's as it's
made, refused at the coercion naming its type, a `chars()` map's (#1563,
`test/diagnostics.test.ts`). `hint_known_made_dyn` (`knowing.rs`) says
which: known as made (`hint_known_as_made`, which `size_hint()` of a
concrete one asks too), or made knowing it by the mark.

Mutations: `dyn-size-hint-unasked`, `dyn-coercions-unmarked` and
`dyn-unseen-unnoticed` (`lower/analysis/hint_asked.ts`),
`dyn-size-hint-refused` (`lower/mir/iter.ts`),
`dyn-coercion-hint-unchecked` (`lower/mir/rvalues.ts`);
`adapter-hint-refused` moved to `lower/iterators/knowing.ts`, of
`hint_known_as_made`, and `hint-refusal-names-no-cause` re-anchored;
`dyn-hint-refusal-names-no-issue` gone with the arm it held, as
`size_hint_of` answers a `dyn`'s itself. Guide: limitations.md.

## Amendment (2026-11-14, #1563): what keeps std's hint by its form, made a `dyn`

Case ([0262](0262-when-rust-and-js-disagree.md)): none; a refusal lifted,
each hint std's.

Amended above (#1476), a value made a `dyn Iterator` in a crate that asks
a `dyn`'s hint was refused where its hint isn't known as it's made: a
`map` of a string's `chars()`, `Box::new(s.chars().map(f))`. But such a
crate makes a `dyn` of what holds a `Chars`, so it makes every `chars()`
the runtime's `$chars` (#1186, `chars_whole`), whose hint is std's, and
`$chars`' own `map`, `rev()` and `take(n)` pass it on: lengths.rs's
`hint_kept_by_form` says so of a type, the rule a `Vec`'s growth already
asks (#1572). `hint_known_made_dyn` asks it too, of the crate's
`chars_whole`, so a `map` of a `chars()`, of a `char_indices()` or of a
`rev()` of one compiles, its hint std's. A `filter` of one, whose hint the
form rule doesn't say, and a `skip(n)`, JS's own `drop(n)` of `$chars`,
which keeps none (#1316), stay refused (#1563).

Held by `test/corpus/dyn_iterator_of_chars_size_hints.rs`, against native
Rust in both builds, of non-ASCII text, where a hint's lower bound isn't
its count; the diagnostics' refusal of a `filter`; and the mutation
`dyn-hint-of-form-unknown`.

