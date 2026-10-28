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
