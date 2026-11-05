# 0997. A zip std folds otherwise than it steps is std's struct

Status: Accepted. Fixes #380 where rustc knows the sides' types whole.
Amends [0975](0975-std-s-zip-is-compiled-its-impl-picked-by-its-sides-facts.md),
and narrows the guide's semantics difference 15
([0139](0139-lazy-chains.md)) to generic code.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. A
closure ran once more than Rust runs it.

## Context

```rust
v.iter().map(|&x| { calls += 1; x }).zip(vec![7, 8]).collect::<Vec<_>>()
```

std's `Zip` of sides both `TrustedLen` but not both `TrustedRandomAccess`
folds by `SpecFold`'s `TrustedLen` impl: as many pairs as its exact size
hint says, the shorter side's. Its `next()` steps `a` first, so a `for`
runs `a`'s closure for the longer side's extra item, and `collect()`
doesn't: 3 calls in a `for`, 2 in `collect()`. rust-js gave 3 for both.

Four things stood in the way:
1. `Zip` was one of the adapters made as their meaning, std's struct in
   the `--std-iterators` build alone (ADR 0932).
2. A zip made in place was the recognizers' `$lazyZip`, which can't tell
   a fold from a `next()` loop.
3. rustc resolves `SpecFold::spec_fold` of a `Zip<A, B>` to its default
   impl, a `default fn` specialized on a marker trait. rust-js took that
   resolution as final, so std's `TrustedLen` impl was never compiled.
4. `collect()`, `fold` and `last()` of std's struct stepped it by `next()`:
   `.toArray()`, `.reduce`, `.toArray().at(-1)`.

## Decision

1. `Zip` leaves `MEANING_ONLY`. A held zip is std's struct in every build,
   as a held `map` is.
2. A zip of sides both `TrustedLen` but not both `TrustedRandomAccess` is
   std's struct where it's made too, where the extra step would show: `a`
   holds a closure or a `fn`, or gives items with a destructor
   (`folds_apart`, of recognition, read by the in-place analysis). That's
   asked of types rustc knows whole. Elsewhere it's `$lazyZip` as before,
   since an extra step that runs nothing can't be seen.
3. Where a fact specialization exists and rustc's resolution is its
   selected default, the facts pick, at the call (`calls.rs`) and in what
   `@rust-js/std` compiles (`std_items.rs`). `SpecFold`'s `TrustedLen` impl
   is compiled, `zipSpecFoldTrustedLenTrustedLen_specFold`.
4. Std's struct's `fold` is its compiled impl's, with its facts.
   `collect()` into a `Vec` of a `TrustedLen` one is `$foldToArray(it)`, by
   its `fold`, as std's `extend_trusted` collects. `last()` of one, of
   items with nothing to drop, is `$last(it)`, by its `fold`, as std's
   default `last` is.

## Consequences

The normal build is 216 lines longer over 26 snapshots: held zips and
zips that fold apart are std's struct, with its dictionaries. A lowering
test of `$lazyZip`'s order now zips a `filter`, which isn't `TrustedLen`.

`test/corpus/zip_fold_steps_the_shorter.rs` checks `collect()`, a `for`,
`sum()`, `count()`, `for_each`, `last()` and a held zip's `fold` against
native Rust, in both builds. The mutations are
`zip-folding-apart-in-place`, `trusted-collect-by-next` and
`compiled-last-by-next`.

Generic code's zip, whose sides' traits only its caller knows, still
steps as before: difference 15 narrows to it.

## Amendment: a map or an enumeration of such a zip is std's struct too (2026-11-05, #654)

Case ([0262](0262-when-rust-and-js-disagree.md)): R, results change, so
Rust's behavior is kept. The output form is the user's decision
(2026-11-05), taken on the recommendation below.

`ds().into_iter().rev().zip(10..13).map(f).collect::<Vec<_>>()` dropped
the reversed `Vec`'s rest back to front, where std drops it as one slice,
front to back. The zip folds apart, so it was std's struct where it was
made, but the `map()` over it was made in place as a JS `map`. The
collect stepped that `map` by `next`, where std's `extend_trusted` folds
it, and `Map::fold` is the zip's `SpecFold`, which takes the shorter
side's count of pairs. Stepped by `next`, the zip takes one item more of
its first side and drops it before the rest.

Whether the zip is std's struct is a fact of its own call site, which the
collect can't see. So it's carried by the type, not read from the `map`'s
form: `recognition::folds_apart`, the one answer, now also holds of a
`TrustedLen` adapter whose fold is its source's, each item mapped, a
`map()` or an `enumerate()`, whose source folds apart. An `inspect()`
isn't `TrustedLen`, so std's collect steps it by `next` too, and a
`copied()` of a zip's pairs can't be written. Such an adapter is std's
struct where it's made too (`in_place_adapters` leaves it out). A collect
folds it by `$foldToArray`, as it folds the zip.

The two corpus cases that already ran as Rust does read longer. In
`zip_fold_steps_the_shorter.js`, a `.map(f).reduce(..)` of such a zip is
now `iterator_map(iterator_zip(..), f, dict).reduce(..)`. In
`compiled_adapters_closed_as_a_collect_unwinds.js`, a
`$foldToArray(zip.map(f))` is now `$foldToArray(iterator_map(..))`,
closed in a `finally`. Leaving the form, with the collect stepping a JS
`map`, would have needed the collect to read the `.map(f)` form, which
reconstructs a fact. It is the representation #380 chose for the zip
itself, and it arises only where a zip folds apart.

`zip_mapped_collect_folds.rs` checks a reversed and a forward `Vec`'s zip
mapped, an enumeration of one, each collected, and an inspect of one
collected and folded, against native Rust in both builds. The matrix's
#654 list is empty. Mutations: `map-of-a-zip-folded-apart-made-in-place`
and `enumerate-of-a-zip-folded-apart-made-in-place`
(`lower/recognition/std_items.ts`).
