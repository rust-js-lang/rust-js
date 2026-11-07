# 1102. What's made in turn drops as a later part unwinds

Status: Accepted (#897, #898, #902, #903; the form approved by the
coordinator for the user, 2026-11-08). Amends
[0052](0052-std-trait-impls.md) (a derived or std `Clone` and `Default`
rust-js writes), [0507](0507-a-vectors-edits-of-destructors.md)
(`resize`, `fill`) and [0509](0509-more-std-calls-of-destructors.md)
(`unwrap_or_default`'s `Err`).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behaviour is preserved: a drop that runs natively and not in JS is
a line, a count or a resource the program sees.

## Context

#854's unwind table runs each operation beyond iterators once each point
a panic can be, every make, clone, drop and comparison of a `D`, against
native Rust in both builds. Its first run found 163 cases, 326
case-builds, of one mechanism in four issues: **a value made of parts,
whose making of a later part panics, leaks the parts it made already.**

- #897: a derived `Clone`'s fields and a tuple's; a clone of a `Vec`, a
  `VecDeque`, a `BinaryHeap`, an array, a `BTreeMap` or a `HashMap`, its
  items; `to_vec` and `extend_from_slice`'s items' own clones.
- #902: a derived `Default`'s fields and a tuple's: `mem::take`,
  `unwrap_or_default`.
- #898: the value `resize`, `fill` and `vec![x; n]` clone from.
- #903: what `inspect` and `inspect_err` give back, as their closure
  panics; an `Err` `unwrap_or_default` drops, as `T::default()` panics.

In Rust each is the same rule, which drop elaboration gives every body:
what a frame has made, a temporary of an aggregate's or a local, is
dropped as a later call in it unwinds. The MIR rust-js lowers has those
edges, so a struct literal the program writes is right (the table's
`value struct` rows pass). None of these four is lowered from MIR:
`clone_value` and `default_value` (`std_impls.rs`, `std_impls/defaults.rs`)
write a derived or std `Clone` and `Default` as an expression,
`{ ...p, a: clone(p.a), b: clone(p.b) }` and `items.map(clone)`; the
runtime's `$resize`, `$fill` and `$repeat` loop by hand; `inspect` is an
`if` around the call. Each wrote the parts in order and no unwind edge.

## Decision

**One rule, asked where rust-js writes a value of parts itself: a part
with a destructor made before a later part whose making may panic is
dropped as that one unwinds.** Its owner is `std_impls`, its rule `std_impls/in_turn.rs`:

- `clone_may_panic` (moved there from `mir/iter/collect.rs`, its one
  answer, which a `cloned()` iterator's steps ask too) and
  `default_may_panic` say whether making a part may panic. A `Copy`
  value's clone never does, nor an `Rc`'s or a `Weak`'s, which counts; a
  hand-written impl's may, unless each of its methods never unwinds (ADR
  0902); a derived one's as its fields' do (before, `clone_may_panic`
  answered "never" of every derived `Clone`, so a `cloned()` of a derived
  type took its clone to be quiet); std's types' as what they hold's do,
  except an empty collection's `Default`, which makes none.
- `held_in_turn` says which parts are held: each with a destructor made
  before the last whose making may panic. Where none is, the value is
  written as it always was.

Two forms, one per order Rust drops in:

- **Parts of a fixed number**, a derived `Clone`'s or `Default`'s fields,
  a tuple's, a map entry's key and value, an array's `Default`'s items,
  dropped the latest first, as Rust drops an aggregate's operands:
  `$madeInTurn((made) => ({ ...p, a: made(clone(p.a), drop), b:
  clone(p.b) }))`. `made(part, drop)` gives the part back as it holds
  it, and `$madeInTurn` drops what it holds as `build` throws. A field
  that needs no clone of its own, a `{ ...p }` copy, is held too where it
  drops, as the derived `Clone` made one.
- **Items**, a clone of a slice's, a `VecDeque`'s, a `BinaryHeap`'s, an
  array's or a map's entries, dropped in order, as std's clone drops what
  its guard holds: `$madeEach(items, clone, drop)`, `Array.from`'s of
  `clone` but for that.

Both are `src/runtime/made.js`'s, and each asks `$unwinds`, the
runtime's one guard of what drops as a panic unwinds, whose
`$dropsUnwinding` is the one place a drop's second panic aborts
(`test/architecture-iterators.test.ts` holds it so). The runtime's own
loops ask it too: `$resize` holds
the value it was given across a drop of what it cuts off, the room check
and each clone; `$fill` across each clone and each drop of what it
replaces; `$repeat` across each clone, then drops the clones made, as
`extend_with`'s frame unwinds before `from_elem`'s drops its `Vec`.
`inspect` and `inspect_err` drop the value they give back as their closure
unwinds, where it may (`call_may_unwind`, ADR 0902's answer, which a
fold's stages ask too): `try { f(x); } catch (error) { drop(x); throw
error; }`, `drops::dropped_unwinding`'s. `or_dropped`, of
`unwrap_or_default`'s `Err` and its kind, holds what it gives up across
what it makes otherwise where that may throw.

## Alternatives

- **A named clone function per type**, `function cloneP(p) { const a =
  clone(p.a); const b = $unwinds(() => clone(p.b), () => drop(a));
  return { ...p, a, b }; }`, as a recursive type's is (`recursive_fns`).
  It reads more like JS a person writes. But `clone_value` and
  `default_value` make expressions, under an `Option`'s `p == null ? p :`
  and an enum's variant tests, where no statement can go before the
  part; each such type would need its function, an enum's of `if`s, and
  `default_value` its statements, which it has none of. The helpers make
  the guard an expression wherever the clone already is. Not chosen.
- **Lower a derived `Clone` and `Default` from their MIR**, whose cleanup
  edges are rustc's. That gives up rust-js's spread clone, which shares
  what nothing changes (ADR 0020), for `const value = ..; let value$live`
  of every field, and std's `Vec` clone has no MIR rust-js can lower. Not
  chosen.
- **Guard every clone of parts with a destructor.** Simpler to decide;
  but most clones can't panic, and `Vec<String>`'s would read worse for
  nothing. Not chosen: the guard is only where `held_in_turn` says.

## Consequences

- #897's, #898's, #902's and #903's rows of the unwind table pass, 306
  case-builds, but `Vec clone_from`'s and a `fill` of a `Vec` of `Vec`s:
  a `Vec`'s `clone_from` runs std's `SpecCloneIntoVec`, another order of
  clones and drops with no panic at all, filed as #919.
- One corpus snapshot changed, as Rust's behaviour needs:
  `refcell_generic_borrow_meets_held`'s clone of a `Vec<Node>`, whose
  `Clone` is hand-written and whose drop drops a `Weak`, is `$madeEach`.
- `test/lowering-made-in-turn.test.ts` holds both forms: none of `Copy`,
  `String` and `Vec<String>` fields, and of a derived `Clone` of them;
  each guarded form of a panicking `Clone` and `Default`.
- Corpus: `clone_unwinds_what_it_made`, `default_unwinds_what_it_made`,
  `resize_fill_repeat_unwind`, `inspect_unwinds_its_value`, each failing
  before in both builds.

## Amendment: std's own methods keep their state as a drop in them unwinds (2026-11-08, #899, #900, #904, #906, #919)

The unwind table's second cluster: a std method rust-js writes itself,
not from MIR, that drops or clones many values left its collection as it
was, not as std's guard leaves it, when one of those panicked. The rule is
this ADR's, said of state as well as of what's made: **what std's method
has written, emptied or taken stays so as a drop or a clone in it
unwinds, and what it holds is dropped as it unwinds.** Each asks the one
answer of whether a drop may panic, `drop_may_panic` (ADR 0902's), or how
an item is written over, `clone_over`; where nothing may panic or be seen,
the form is as it was. The approved forms (the coordinator for the user,
2026-11-08):

- **`clear()` of a map or a set** (#899): its entries dropped as dropping
  the map drops them, the rest though one panics (`$inTurn`'s, a second
  panic aborting, #847), with `m.clear()` in a `finally`, as std's
  `BTreeMap::clear` takes them out before it drops them and `HashMap`'s
  guard clears the table as a drop unwinds: `try { <drop m> } finally {
  m.clear(); }`, `drops::dropped_after`'s one form. Before, `<drop m>;
  m.clear();` left the map full, each entry dropped again with it.
- **`extend` of a `Vec` or an array into a map or a set** (#900):
  `$extendMap` drops what's left of it as a put unwinds, a drop of what it
  replaces or of the key it was given, each entry its key then its value
  (`$entryDropping`, which `$mapFrom` asks too), as std's `for_each` drops
  the `IntoIter` it took. A JS iterator was closed by `for..of` already.
- **`Option::insert`** (#904): `*o = Some(v)`'s drop in place of the old
  value writes `v` as it unwinds too: `try { if (o != null) { drop(o); } }
  finally { o = v; }`. Before, the old value stayed, dropped again with
  `o`, and `v` leaked.
- **`dedup_by`, `dedup_by_key` and `pop_if`** (#906): the cells helper is
  chosen by how a `&mut` to an item is given (`MutRefForm::is_cell`), not
  by whether it's an object, so an `Rc`'s closure is given the cell it
  reads (before, a `TypeError`); `$dedupByCells` is `$dedupBy`'s loop, so
  a `Box`'s or an `Rc`'s duplicates are dropped as they're found (before,
  never), and each cell is written back as the closure returns or unwinds.
- **A `Vec`'s `clone_from`** (#919): std's `SpecCloneIntoVec`,
  `$cloneIntoVec(w, v, clone, cloneFrom, drop)` in place: `truncate`, each
  item kept written over by `clone_over`'s answer (its own `clone_from`, a
  `Vec`'s again, or a clone then the old one's drop), then a clone of each
  of the rest pushed. `clone_over` knows a `Vec`'s `clone_from` too, so
  `fill` and `clone_from_slice` of `Vec`s of `Vec`s write each so, and of
  `Vec`s of a type with its own `clone_from` are no longer refused. Of
  items with no destructor and no `clone_from` of their own, `String`s,
  nothing sees the order: `*w = v.clone()`, as before (0262's "none").

Case (0262): results change; Rust's behaviour is preserved. Corpus:
`map_clear_unwinds_emptied`, `map_extend_unwinds_what_is_left`,
`option_insert_unwinds_written`, `dedup_by_drops_as_it_finds`,
`vec_clone_from_in_place`, each failing before in both builds;
`clone_from_slice_nested` compiles where it was refused, and
`clone_from_slice_nested_option` keeps the refusal of an `Option`'s.
`test/lowering-std-unwinds.test.ts` pins each quiet and guarded form. The
unwind table's 128 case-builds of the five issues pass, and 4 of #911's,
a `clear` of a B-tree of enum keys.

Found and filed, not fixed here: #920, a `HashMap`'s or `HashSet`'s drop
goes on past an entry's panicking drop where hashbrown's stops; #921, a
`BTreeMap`'s `insert` or `extend` of a key already there writes the value
before dropping the key it was given; #922, std's other `clone_from`s
(`VecDeque`, `Option`, `Box`, the maps).

## Amendment: std's other `clone_from`s (2026-11-08, #922)

#919 made a `Vec`'s `clone_from` std's; std writes its own for other types
too, and rust-js lowered each as `*dest = source.clone()`, every clone and
then the old value dropped whole: another order of clones and drops, with
no panic. Results change (0262); Rust's behaviour is preserved.

**One answer of how a value is written over in place by a clone of
another, `clone_into` (`std_types/slice.rs`, beside `clone_over`), asked
by `clone_from` and by `clone_over`, so `fill`, `clone_from_slice` and an
array's items follow it too:**

- a `Vec`'s and a `BinaryHeap`'s, `$cloneIntoVec`, as #919;
- a `VecDeque`'s, `clear()` then a clone of each pushed,
  `$cloneIntoDeque(w, v, clone, drop)`;
- a `LinkedList`'s, each it keeps written over (its items' `clone_from`),
  the rest popped from its back, then the rest cloned on,
  `$cloneIntoList`;
- an array's, its slice's `clone_from_slice`, `$cloneFromSlice`;
- a `HashMap`'s or a `HashSet`'s, hashbrown's: its entries dropped, a panic
  stopping that and leaking the rest, the table emptied (`$dropTable`,
  which #920 makes a hashed map's drop too), then each entry cloned as a
  map's clone clones it (`entry_clone`, `clone_parts`' one answer), those
  made dropped as a later one panics, `$cloneIntoMap(w, v, make, drop)`;
- a `Box`'s, its value's.

An `Option`'s is its value's where both are `Some`, and otherwise
`*dest = source.clone()`: `if (w != null && v != null) $cloneIntoVec(w, v,
..); else { <replaced> }`, at a `clone_from` and as an item's in
`clone_over`. A `BTreeMap`'s and a `BTreeSet`'s are std's provided
`*dest = source.clone()`, as before.

Where the order is seen now includes a clone that may panic: of items with
no destructor and a clone that panics, std's leaves `w` half written,
which `*w = v.clone()` doesn't, so `$cloneIntoVec` and the others are used
there too, a change to #919's rule. Where nothing drops and no clone may
panic, the form is as it was.

**Refused, not approximated:** a `clone_from` of an `Option`, a `Result`, a
`Box` or a `Reverse` whose value's `clone_from` is the crate's own, or one
`clone_into` writes in place where rust-js doesn't write its forwarding (a
`Result`'s, a `Reverse`'s, an `Option` of a boxed payload):
`forwards_clone_from`, held by `test/corpus/clone_from_option_own.rs`.
Before, each was `*dest = source.clone()`, which never called the crate's
`clone_from`; the refusal says what to write instead, and #927 asks for the forwarding. A generic `Option<T>`'s is still `*dest = source.clone()`, #928.

Corpus: `clone_from_std_own`, each of the types under clones that panic,
with and without destructors, against native Rust in both builds, failing
before; mutations `clone-into-*`, `clone-from-*`, `option-items-cloned-over`,
`vec-clone-into-panic-unseen`, `drop-table-leaks`; and the guide's
`queue_into`.

## Amendment: a hashed map's drop stops where an entry's panics (2026-11-08, #920)

hashbrown's `RawTable` drops its elements in a plain loop (`drop_elements`,
in its `Drop` and in `clear`'s guard). An entry whose drop panics stops it,
and the rest leak. rust-js dropped a `HashMap`'s or `HashSet`'s entries with
`$dropItems`, which goes on past a panic as a slice's drop glue does. Case
C ([0262](0262-when-rust-and-js-disagree.md)): Rust's behaviour is kept.

**A hashed map's or set's entries drop in a plain loop, `for (const item
of m.values()) { dDrop_drop(item); }`, which a throw stops** (`drops/parts.rs`,
`loop_dropped`). A `BTreeMap`'s still goes on past it, as its `IntoIter`'s
guard does (`$dropItems`). `clear()` is `try { <drop> } finally { m.clear();
}` as #899 made it, so it now stops and empties as hashbrown's guard does.
`$cloneIntoMap`'s drop of what it held is the same rule, `$dropTable`.

Held by `test/corpus/hash_map_drop_stops_at_a_panic.rs`: values alone,
keys and values, a set, `clear()`, and a B-tree's beside them, each entry
dropping alike so a hashed map's order doesn't show. Mutations
`hash-map-*drop-goes-on`.
