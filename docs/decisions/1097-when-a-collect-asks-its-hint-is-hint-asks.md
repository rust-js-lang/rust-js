# 1097. When a collect asks its hint is `hint_asks`'s

Status: Accepted (2026-11-06, #820 step 1; step 2's #848 and #849's reserves). On ADR 1096's
model, as #819's was.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. This changes who
answers, not what's answered: every corpus snapshot is byte-identical, in
both builds.

## Context

Six P0s in a month were collects that asked their iterator's `size_hint`
at the wrong time, or not at all, where it runs the crate's own code:
#724 #728 #733 #794 #796 #797. Each was decided where the collect was
lowered:

| what | where |
|---|---|
| whether asking runs the crate's own, `writes_size_hint`, `hint_reaches_own` | `recognition/lengths.rs` |
| which destinations std grows as a `Vec`, `grows_as_vec` | `recognition/types.rs` |
| whether a `Vec`'s collect asks room, `$roomForHint`, `TrustedLen`, endless, endless once stepped, the first item's drop | `mir/iter/collect.rs`'s `collected_as_vec` |
| the size its items grow it by, `TSize` where it's a caller's, `grown_item_size` | `mir/iter/collect.rs` |
| the same, asked of the caller, `grown_asked` | `analysis/type_facts.rs` |
| a `String`'s and a hash table's reserve, `$reserveHint`, `reserved_by_hint` | `mir/iter/collect.rs` |

The lowering's `grown_item_size` and the analysis's `grown_asked` were
two conditions of one rule, kept alike by hand.

## Decision

One owner, `iterators/hint_asks.rs`, answers when a collect asks its
iterator's hint and for what room, as std's paths do: `SpecFromIter` of a
`TrustedLen` iterator, its hint first; `SpecFromIterNested` of another,
its first item then its hint's lower bound and one more; and
`extend_desugared` as the `Vec` fills, asking it again; and the reserve a
`String`'s or a hash table's `extend` makes first.

- `grown_ask` says what a collect into a `Vec` std grows asks, a
  `Grown` of its items, whether it's `TrustedLen` and whether it's
  endless, or nothing; `grown_by_hint` makes `$roomForHint` of it, of a
  caller's `ITrustedLen` where that's its caller's.
- `reserved_by_hint` makes `$reserveHint`.
- `grows_by_callers_size` is the one answer of whether a grown `Vec`'s
  items' size is its caller's `TSize`: the lowering's `grown_item_size`
  and the analysis's `grown_asked` both ask it.

`test/architecture-iterators.test.ts`'s "when a collect asks its hint is
hint_asks's alone" holds it: no module but the owner makes `$roomForHint`
or `$reserveHint`, and none but it asks `hint_reaches_own`, but
`Filter::count`'s (`mir/iter.rs`), which is no collect.

## Step 2: through std's `GenericShunt` (#848)

A collect into a `Result` or an `Option` of a collection, `try_process`,
collects a `GenericShunt` of its source, whose hint is its source's, its
lower bound none, `(0, upper)`, asked while no `Err` or `None` has
stopped it. rust-js asked none. The owner's `shunted_ask` answers it, a
`Shunted`: of a collection std grows as a `Vec`, as `SpecFromIterNested`
and `extend_desugared` ask it, after the first value, then each time the
`Vec` is full, `$collectResults` and `$collectOptions` given the values'
size, a caller's `TSize` where they're its caller's
(`grows_by_callers_size` sees through the `Result` or `Option` to them);
of a `String` of `char`s or a hash table, once, first, room for none,
`$reserveHint(it)` (`Room::Nothing`); of a `LinkedList` or a `String` of
strings, never. Only where asking it may run the crate's own. Where
asking it panics, the value just taken drops first, as
`extend_desugared`'s element does. std's `MIN_NON_ZERO_CAP` is one
runtime function, `$leastCapacity`, of both `$roomForHint` and the
shunt's growth.

Case ([0262](0262-when-rust-and-js-disagree.md)): results changed, so
Rust's behaviour is kept. Held by corpus `collect_into_results_asks_hint`
(each destination, a failure first and later, generic values of a size
and of none, and a hint that panics first and later, with values that
drop) against native Rust in both builds; two generic snapshots now
give their collect its values' size, `generic_collect_closes_its_source`
and `generic_collect_drops_rest_first`, as a type parameter's hint may
be the crate's own (ADR 1092). Mutations `shunted-hint-unasked`,
`shunted-callers-size-zero`, `shunted-reserve-unasked`,
`shunt-growth-unasked`, `shunt-growth-not-doubled` and
`shunt-hint-panic-drops-not-taken`.

## Step 2: an `extend` that reserves first (#849)

An `extend` asked no hint. std's of a `String` by `char`s reserves its
lower bound first (string.rs `Extend<char>`, `&char` too); hashbrown's,
a `HashMap`'s and through it a `HashSet`'s, reserves its lower bound
where the table is empty, else half of it rounded up. The owner's
`extended_reserved` makes `$reserveHint(it, size, align, of)` of them,
counted with what `of`, the collection, holds (`$textRoomFor`, and the
table's entries); of a `String` by strings, or a B-tree's, none. A map
not named, `new Set()`, is held first only where it's reserved
(`extend_reserves`), so nothing else reads otherwise. Case
([0262](0262-when-rust-and-js-disagree.md)): results changed, so Rust's
behaviour is kept. Held by corpus `extend_reserves_by_hint` against
native Rust in both builds, and the mutations `table-extend-hint-unasked`,
`map-extend-unreserved` and `string-extend-unreserved`.

## Next

- Step 2's rest, the `extend` that asks as it grows (#849), a corpus
  case against native Rust in both builds: a `VecDeque`'s, a hash table's and a
  `String<char>`'s reserve once; a `Vec`'s asks as it fills, exact where
  its capacity is known where it's extended, made in the same body by
  `Vec::new`, `with_capacity`, `vec![..]` or a collect and only extended
  or reserved since, refused elsewhere (the user's (B), 2026-11-06).
- Step 3, the conformance table's rows: each destination, collected and
  extended, of each kind of source, `TrustedLen` std struct, in place,
  the crate's own hint and a type parameter's.

## Tests

Byte-identical: the corpus in both builds. Mutations: the ten of
`collect.ts` and `type_facts.ts` moved to `lower/iterators/hint_asks.ts`
and retargeted, each caught (`grown-size-asked-of-a-known-hint` now of
the snapshots, as the analysis and the lowering change together), and
`string-or-table-hint-unreserved`.
