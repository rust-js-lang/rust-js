# 0493. A zip of arrays' iterators steps from its back

Status: Accepted. Extends [0164](0164-double-ended-iterators.md) and
[0429](0429-a-double-ended-iterator-knows-its-back.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): the pairs, in the
order Rust's `Zip::next_back` gives them.

## Context

`rev()` of a lazy iterator is refused (ADR 0164): its closures would run
in another order. A zip of two arrays' iterators runs none, but was
refused with them: itertools' `MultiProduct` walks
`iters.iter_mut().zip(values.iter_mut()).rev()`.

## Decision

**`rev()` of a zip of two arrays' iterators, made here, is `$zipBack(a,
b)`: the longer trimmed from its back to the shorter's length, then each's
back paired, as Rust's `Zip::next_back` does where both sides are
double-ended and exact-sized.** Of `iter()`'s and `iter_mut()`'s alike,
`$iter`s, which know their backs and lengths. `reversed`, the one place
`rev()` steps from a back, makes it.

- **Not of items with a destructor**, which trimming would drop: refused,
  as before.

```js
for (const [x, y] of $zipBack($iter($mutItems(a)), $iter($mutItems(b)))) {
```

## Why

- **It's std's**: `zip_from_back` reverses zips of a longer first side, a
  longer second, an empty one, and changes items through `iter_mut()`,
  breaking early, with native Rust.

## Consequences

- **What compiles grows**: itertools' `multi_cartesian_product`.

## Amended (2026-11-05, #740): any zip whose sides know their backs and lengths

`v.iter().copied().zip(0..2).rev()` and its `cloned()` form were refused,
"a `Zip<Copied<Iter<u32>>, Range<i32>>` as a `DoubleEndedIterator`", as
were `len()` of a zip, a `rev()`'s `nth_back` of one (#718), and a zip of
two `cloned()`s of a `Drop` item asked its length: `$zipBack` stepped two
arrays' `$iter`s alone, of items with nothing to drop. **A zip made here,
`$lazyZip(a, b, ..)` or one std reads by index, `$trustedZip(a, b)` (ADR
0973), is made knowing its back and length where it's asked them,
`$zipEnds(a, b, dropA, dropB)` or `$trustedZipEnds`, of its sides each
made so** (knowing.rs's `zip_knowing`, of `as_double_ended` and
`as_exact_size`): its length the shorter's, and its back std's
`Zip::next_back`, the longer trimmed from its back to the shorter's
length, each it trims dropped, then each's back paired, the first's item
dropped where the second's step panics. One the crate steps from its back
in a variable is made so where it's made (`back_stepped`, ADR 0545), and
a `rev()` of one is `$rev` of it, made knowing its back too. Its `len()`
made here is the fewer of its sides', `Math.min(v.length, w.length)`
(`len_in_place`).

`$zipBack` is gone, the user's decision (through m6-react): a `rev()` of
a zip is `$rev($zipEnds(a, b))`, the one form, `rev_items`' of what's made
knowing its back; `test/architecture-iterators.test.ts`' "a reversed zip
has one form" refuses a `$zipBack` brought back. Value.rs's
`zip_parts` reads what a zip is given, and `zip_ends` makes it, as it owns
each iterator form (ADR 1092).

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable; a
refusal of valid Rust lifted, each item given and dropped where std's
are. Held by corpus `zip_from_its_back`, against native Rust in both
builds, and `zip_from_back`. Mutations `zip-not-made-knowing-its-back`,
`zip-back-trims-undropped`, `zip-not-counted-held`,
`rev-of-zip-not-made-knowing` (`iterators/knowing.ts`),
`zip-not-back-stepped` (`analysis/back_stepped.ts`), `zip-len-the-longer`
(`mir/iter/sources.ts`), and `zip-ends-trim-keeps-a`,
`zip-ends-back-throw-keeps-a`, `zip-ends-len-longer` (`runtime/lazy_zip.ts`),
with `zip-back-untrimmed` on the new trim. #400's rule, a
`size_hint()` of a `rev()` sized in place only where what it's made of is,
is gone with its mutation, `rev-sized-whatever-its-source` (the user's
decision, through m6-react: no code left that no test holds): it held a
`rev()` of a zip apart as `$zipBack` made in place had no hint, and
`$rev($zipEnds(..))` has std's. Without it no answer changes, of the corpus
or either iterator slice, and `zip_size_hints`' `rev()` of a zip reads
`$sizeHint($rev($zipEnds($iter(v), $iter(w))))`, not std's compiled
`Rev<Zip>`. A zip of `cloned()`s whose
`Clone` runs code is still refused, "a `Cloned<Iter<D>>` as an
`ExactSizeIterator`": a `cloned()` of a slice's items made `.map(clone)`
knows no length, #792.

## Amendment: each side's length asked once a step (2026-11-06, #824)

`$zipEndsOf`'s back asked `left.len()` in each of its two trimming loops,
so a zip of the crate's own exact-size iterator stepped from its back,
`Every::new(0).zip(10..13).rfold(..)`, ran the crate's `size_hint` twice a
step, and once more after it trimmed. std's `Zip::next_back` asks `a.len()`
then `b.len()` once each before it trims. Each side's length is now read
once a step, `a`'s then `b`'s. ADR 0262: results change (the crate's code
runs as often as natively), so Rust's is kept. Held by
`zip_of_own_from_its_back_asks_each_len_once.rs` and the conformance
table's 24 `own Every | zip` rows; mutation `zip-back-len-asked-per-trim`.
First bad commit 58af383ef.
