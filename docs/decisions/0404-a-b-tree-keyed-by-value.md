# 0404. A B-tree keyed by value, whose `Ord` is derived

Status: Accepted. Amends [0121](0121-value-keys.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a B-tree finds a
key by its `Ord`, so `$key`'s equality must be `Ord`'s, which a derived
`Ord` all through makes it; where it isn't, D: a compile error.

## Context

ADR 0121 made a map keyed by a tuple or a struct a `$KeyMap`, but refused
a `BTreeMap` or `BTreeSet` of those keys, as a string of the key's fields
isn't in the key's order. A B-tree is now a map in insertion order, sorted
by its keys' `cmp_fn` where its order is read (`$sortedEntries`), so its
storage needn't be in order: only its equality must be its `Ord`'s.
`BTreeMap<(i32, i32), _>`, a grid by its cells, was refused.

## Decision

**A B-tree keyed by value is a `$KeyMap` or `$KeySet`, read in its keys'
order, where the key's `Ord` is derived, or std's, all through:
`orders_by_value`'s answer. Two keys it orders `Equal` are then equal
field by field, as `$key` tells them.**

```js
const grid = new $KeyMap();
grid.set([2, 1], "a");
$sortedEntries(grid, (a, b) => $cmp(a[0], b[0]) || $cmp(a[1], b[1]));
```

- **A hand-written `Ord` anywhere in the key**, a struct's, or a
  fieldless enum's inside a tuple, is refused: it may order `Equal` keys
  that `$key` tells apart.

## Why

- **It's the program Rust runs**: the corpus's `value_key_trees.rs`
  keys B-trees by tuples, by a struct and an enum with fields that derive
  `Ord`, and by a tuple of an `Option` and a `Vec`, and inserts, gets,
  ranges, reads its ends, enters, removes, pops and retains, each with
  native Rust; the diagnostics test refuses two hand-written `Ord`s.

## Amendment: a key whose `Ord` is the crate's own

A B-tree keyed by a struct whose `Ord` is hand-written was refused, as
`$key` may tell apart two keys its `Ord` finds `Equal` (#165). Case A
now: std's B-tree finds a key by its `cmp` alone, and so does the map.

**A B-tree of a key `orders_by_value` doesn't answer for is an
`$ordMap(cmp)` or `$ordSet(cmp)`, `found_by_cmp`'s answer: its entries
kept in `cmp`'s order, each found by a binary search of it.** A key it
finds `Equal` finds the one it holds, as std's does, whatever its fields.

```js
const releases = new ($ordSet(releaseOrd_cmp))();
$storedKey(releases, { major: 2, name: "" });
```

- **The class is made of `cmp`**, so a copy, `new m.constructor(m)`, and
  `split_off`'s new half are ordered by it too.
- **A generic B-tree is given its class by its caller**, as a hashed one
  is (ADR 0466), under its own fact, `TreeMap` or `TreeSet`: a `HashMap`
  of the same key finds it by its fields, a `BTreeMap` by its `cmp`.
- **What gives back a key gives the one the map holds**, `$storedKey`:
  `get_key_value`, a set's `get`, `take` and `replace`, and an occupied
  entry's `key` and `remove_entry`. This lifts ADR 0401's and 0402's
  refusals of those for paths and `Cow`s, which find others too.
- **Made of items, it keeps the last of equal ones** (`$treeFrom`), as
  std sorts what it's made of and keeps the last; a generic B-tree's
  key, which its caller's `cmp` may find `Equal` to another, does too.

Consequences, only where an `Ord` disagrees with its `Eq`, which std
calls a logic error whose results it doesn't specify: std's `from_iter`
keeps two keys `cmp` finds `Equal` that `==` doesn't, and rust-js keeps
one. Held by `test/corpus/btree_own_ord.rs` and `held_keys.rs`, each
with native Rust.

## Amendment: what holds a `$OrdMap`

Case: N ([0262](0262-when-rust-and-js-disagree.md)): what was decided,
held; nothing a program sees changes.

Three mutations held `$OrdMap`, each of finding and keeping a key. Four
more are each caught (an architecture review, 2026-10-25): a key put at
its end, not where its order puts it, by `btree_own_ord.rs`; and, by
`test/runtime-package.test.ts`'s "a map keyed by a struct, hashed or
ordered, reads as a Map to the JS given it", a missing key's removal
taking its neighbour, `size`, and `forEach`'s value before its key, which
JS calls and no Rust program does. A B-tree handed to JS iterates in key
order only where its key has an `Ord` of its own; any other is a `Map` in
insertion order, which Rust sorts as it steps it: #237 asks which the
boundary should be.

## Amendment: a B-tree is read in its keys' order by the JS it's given (#237)

A B-tree of keys a hashed map finds, a number's, a string's, or a
derived `Ord`'s, was a `Map` or a `$KeyMap` in the order it was added,
sorted only where Rust read it (`$sortedEntries`): `ranks()`'s `new
Map([[2, "silver"], [1, "gold"]])` gave a JS `for...of` 2 first, where a
key of its own `Ord`, an `$OrdMap`, gave 1. One type, two orders a JS
caller sees, and the guide's "iterated in key order" true of one. Case C
([0262](0262-when-rust-and-js-disagree.md)): the order a caller reads is
the type's, Rust's, so JS's must be it too.

**A B-tree of keys a hashed map finds is a `$btreeMap(cmp, Base)` or
`$btreeSet(cmp, Base)`, of the class a hashed map of them is, `Map`,
`$KeyMap` or another, read in `cmp`'s order by Rust and by JS alike;
`$BTreeMap` and `$BTreeSet` of keys `$cmp` orders** (`map::btree_class`,
the user's choice of a lazily sorted class). A key added past its last
is in place; one added before it sorts its entries again, in place,
before it's next read, so adding and finding stay a hashed map's, and
reading in order costs a sort once each change.

```js
const ranks = new $BTreeMap([[2, "silver"], [1, "gold"]]);
const grid = new ($btreeMap((a, b) => $cmp(a[0], b[0]) || $cmp(a[1], b[1]), $KeyMap))();
```

- **Rust still reads it by `$sortedEntries`**, as a `Map` JS hands a
  `&BTreeMap` may be in any order: a sort of one already sorted is a
  pass over it.
- **A key of its own `Ord`** is an `$OrdMap` as before, found by it.

Held by `test/runtime-package.test.ts` ("a map keyed by a struct, hashed
or ordered, reads as a Map to the JS given it"), each kind of key read by
JS's `for...of`, `keys()` and `forEach` in key order, one emptied before
it's sorted again read as empty, 64 corpus cases against native Rust,
and the mutations `btree-*` (`runtime/btree.ts`,
`lower/std_types/map.ts`).

## Amendment: a key's own order is asked as std's one node asks it (#911)

`$OrdMap` found a key by halving, asking `cmp(held, key)`, and its callers
asked it again for each question: `insert` was `has`, `get`, then `set`,
three searches; a walk of it sorted it again. A program whose `Ord` prints,
counts or panics saw `cmp 5 2` three times where std's `insert` asks `cmp 2
5` once, and as a comparison panicked, the key `insert` was given leaked,
where std drops it. Results change (ADR 0262): Rust's behaviour is kept,
up to one node.

- **One search, std's argument order, a node's walk:** `cmp(given, held)`,
  from the first key held, to the first not less, as std's
  `search_node`; of up to eleven entries, std's `CAPACITY`, one leaf,
  that's std's sequence exactly.
- **Each operation searches once:** `$found(m, key)` asks it, and
  `$putFound` and `$deleteFound` write what it found without asking again,
  of `insert`, a set's `insert` and `replace`, `remove`, `take`,
  `get_key_value`, `remove_entry`; `split_off` splits where its one search
  stops, as std's split does. Another map, hashed, answers `$found` by
  `has`, which asks the program nothing.
- **A walk asks nothing:** iterating, `retain` (which takes out what it
  walks to by what it is, `deleteHeld`) and `iter_mut`'s handles (on each
  entry itself) ask no `cmp`, as std's walk of its nodes asks none.
- **What a call was given is dropped as its search unwinds**, of keys whose
  order may panic (`order_may_panic`, ordering.rs, the one answer, #901):
  `insert`'s key, then its value, `$insert(m, k, v, dropKey, dropValue)`;
  a set's `insert`'s and `replace`'s item. Of an order that never panics,
  the call is the one it was.

**Its limit, filed:** past eleven entries std's tree has more than one
node, whose shape its splits, rebalancing, bulk build and appends make, and
its sequence follows that shape; `$OrdMap` halves there, in std's argument
order but not its sequence. `entry(k)` asks twice (`$orInsert`, then
`$mutGet`), and `collect` and `append` sort by JS's sort. Each waits on
std's node layer, #926, and its stable sort, #925; a clone of keys with a
`Clone` of their own calls none, #924 (fixed below).

The corpus's `btree_search_order` runs `insert`, `get`, `remove`,
`contains_key`, `split_off`, `retain` and a set's `insert`, `contains`,
`remove`, `take` and `replace` of up to eleven keys whose `Ord` prints and
panics, under panic points, and a map of forty keys of a quiet order;
`test/lowering-std-unwinds.test.ts` pins `insert`'s and `replace`'s forms.
366 of the unwind table's 430 case-builds of #911 pass; its `entry` rows
stay under #911 and its `clone` rows move to #924.

## Amendment: a clone clones each key, and asks no `cmp` (#924)

A clone of a B-tree of keys whose `Ord` is their own shared its keys and
never called their `Clone`: `clone_parts` cloned a key only where a
`$KeyMap` found it by value (ADR 0121), from before a key could be found by
its `cmp` or a caller's class. And it was made by `new ($ordMap(cmp))(..)`,
whose constructor searched for each entry, asking `cmp`, where std's
`clone_subtree` asks none. A `Clone` or an `Ord` that prints showed both,
with no panic. Results change ([0262](0262-when-rust-and-js-disagree.md)):
Rust's behaviour is kept.

- **Each key that needs a clone is cloned**, whatever finds it (`needs_clone`
  alone): its key, then its value, in the keys' order, and the key dropped
  as the value's clone unwinds, then the entries made so far (ADR 1102's
  `$madeInTurn` and `$madeEach`). A key whose clone nothing tells from it,
  a number's or a `Copy` struct's, is the key itself, as before.
- **A B-tree of keys found by their own `cmp`, or its caller's class, is
  made of its entries in their order, asking nothing: `ofSorted`**, a static
  method of `$OrdMap` and `$OrdSet`, and of `$btreeMap` and `$btreeSet`
  (`new this(entries)`, whose `cmp` asks the program nothing), as a generic
  B-tree's class may be any of them:

```js
const w = $ordMap(dOrd_cmp).ofSorted($madeEach(m, ([key, value]) => $madeInTurn((made) => [made(dClone_clone(key), dDrop_drop), dClone_clone(value)]), drop));
const copy = $ordMap(taskOrd_cmp).ofSorted(owners);
```

- **Only a clone makes one**: `ofSorted` trusts its entries' order, so
  `test/architecture-maps.test.ts` holds that `clone_parts`' map arm is its
  one maker and the runtime never calls it.

Exact up to eleven keys, std's one leaf: past it, std's `clone_subtree`
clones a node's edges between its keys, and drops what an internal node
made in another order as one panics (#926). `test/corpus/btree_clone_own_keys.rs`
runs it against native Rust, a map's and a set's, a panic at each key's and
value's clone, and in generic code of keys found by `cmp` and by value;
the mutations `map-keys-*`, `btree-clone-searched`, `ord-*-of-sorted-*` and
`btree-*-of-sorted-empty`. The unwind table's 20 `clone` cases, 40
case-builds, pass.
