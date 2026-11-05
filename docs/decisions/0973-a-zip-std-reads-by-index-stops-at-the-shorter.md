# 0973. A zip std reads by index stops at the shorter

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). Amends
[0139](0139-lazy-chains.md)'s amendment and the semantics' difference 15;
extends [0465](0465-dropping-an-adapter-is-closing-it.md). Leaves #380 open.
Narrowed further by [0997](0997-a-zip-that-folds-apart-is-stds-struct.md):
sides that know their length but aren't read by index fold as std's too.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. A
closure ran once more in JS than in Rust.

## Context

std's `Zip` picks its impl by its sides' traits. Of two
`TrustedRandomAccess` sides (a slice's iterators, an integer range, a
`map`, `enumerate` or `copied` of one) std 1.99 fixes its length at the
shorter's where it's made and reads each pair by its index. `next`,
`nth`, `fold` and `next_back` never step a side past that length. The
general impl's `next` steps `a` first and so makes one more of the first
side's items where the second ends first, running its closure.

`$lazyZip` was the general impl for every zip:

```rust
let v = [1, 2, 3];
let w = [10, 20];
let z: Vec<_> = v.iter().map(|&x| noisy("a", x)).zip(w.iter()).collect();
for p in v.iter().map(|&x| noisy("b", x)).zip(w.iter()) { .. }
```

Natively: `a1 a2`, then `b1 b2`. JS: `a1 a2 a3`, `b1 b2 b3`. ADR 0139's
amendment listed this as rare and harmless (difference 15) because it
read std's `next` of such a zip as stepping the first side too. std 1.99
doesn't (its `next` returns `None` once `index` reaches `len`), so `for`
differed as well as `collect`.

## Decision

A zip whose sides are both `TrustedRandomAccess`, and whose first side
may run code as it's read (std's own `MAY_HAVE_SIDE_EFFECT` of it,
evaluated by rustc: a `map`'s closure or a `cloned()`'s clone), is
`$trustedZip(a, b)`. `b` is made knowing its length. The zip pairs each
side's next while `b` has one, so `a` is never stepped once `b` has none.
That is std's index order: `a`'s item, then `b`'s, up to the shorter's
length.

- `recognition::zips_by_index(a, b)` alone answers which zips these are.
  `zipped` in `combinators/adapters.rs` is the one place that makes a
  zip: a chain's `zip(..)`, `std::iter::zip(a, b)` and a zip cloned
  apart.
- Where `a` can't run code, as a slice's `iter()` or its `copied()` can't,
  the two impls can't be told apart, so the zip is `$lazyZip` as before.
- An argument that's an iterator, a range, reaches `zipped` as it is,
  which makes it know its length (`$iterator(range, .., $rangeLen)`).

Not done here: sides that are only `TrustedRandomAccessNoCoerce`
(`vec::IntoIter<T: Copy>`) or `TrustedLen` (`rev()`, `chain()`). Their
`next` is the general impl's while their `fold` reads only up to the
shorter, so one zip runs its closure differently under `for` and under
`collect()`. A JS iterator can't tell those consumers apart, least of all
through adapters after the zip (#380). The fix is std's `Zip` compiled
as its meaning, its specializations picked by its sides' facts.

## Consequences

`test/corpus/zip_by_index.rs` checks against native Rust `collect`,
`for`, `next()` and `nth` of a slice's and a range's `map` zipped, both
sides mapped, `zip(&w)`, `std::iter::zip`, and a `filter` that isn't
random access. The mutations are `zip-by-index-unasked`,
`zip-by-index-steps-a` and `zip-by-index-pure`.

## Amendment: a zip's `b` that holds what drops is made knowing its length where it's made (2026-11-05, #775)

Case ([0262](0262-when-rust-and-js-disagree.md)): none, since this was a
refusal. Rust's behavior is kept.

`v.iter().map(f).zip(w.iter().map(g))` was refused in the default build
where `g` holds a value with a destructor: "a `Map<Iter<u32>, ..>` as an
`ExactSizeIterator`". `$trustedZip` asks `b`'s length. MIR holds such a map
in a variable to drop its closure, so `zipped` had no form of `b` left to
make knowing its length. ADR 0545's #385 rule makes such a map knowing its
length where it's made, but only where a bound asks it, and a zip by index
has no bound of its own.

`back_stepped` now records `b` where a call of `Iterator::zip` has sides that
`zips_by_index` reads by index and `b` is a map whose closure drops
(`closure_drops`). That is the same answer the bound's rule gives. A
range's map is made knowing its length too: `knows_its_length` says so of a
back-stepped map of a range whose closure has a destructor, as it says of
an array's, and `back_kept` makes its range `$iterator(range, ..,
$rangeLen)` where the map is made. A back-stepped range map whose closure
drops nothing is made in place, where its form is read, so its JS doesn't
change.
Without that second step, the map claimed a length its JS didn't have, and
`$trustedZip` threw `b.len is not a function`. That was caught before it
landed and is held by the case below.

Kind: missing case. Area: iterators. Old.
`zip_by_index_of_holding_maps.rs` checks `next`, `len`, `next_back`, a
fold whose `b` is a range's map, a collect, a range's map that holds
nothing that drops, made in place as before, and a map that panics part
way.
It runs against native Rust in both builds. Mutations:
`zip-by-index-holding-map-unasked` and `zip-by-index-any-map-asked`
(`lower/analysis/back_stepped.ts`),
`back-stepped-range-map-length-unknown` and
`back-stepped-range-map-length-always` (`lower/iterators/knowing.ts`),
`range-map-made-lengthless` (`lower/mir/iter/backs.ts`).
