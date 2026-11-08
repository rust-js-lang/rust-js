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

## Amendment: std's nodes, where a key's order can be seen (#926)

The amendments above were exact up to eleven keys, std's one leaf. Past
that, std's tree has more than one node, and the comparisons a search asks
follow the nodes' shape: `$OrdMap` halved there. `entry` searched twice,
and `collect` and `append` sorted with JS's `sort` and searched each key.
A program whose `Ord` prints, counts or panics, or isn't a total order,
saw other calls, other drops, and of an order that isn't total, another
order. Results change ([0262](0262-when-rust-and-js-disagree.md)), so
Rust's behaviour is kept.

- **The class:** keys found by their own `cmp` whose comparisons can be seen
  (`order_seen`) or whose order may not be total (`order_total`, #936) are
  `$ordTreeMap(cmp)` and `$ordTreeSet(cmp)`: `$OrdTree`, src/runtime/ord_tree.js,
  a port of alloc/src/collections/btree (rustc 1.99). Its nodes hold up to
  eleven entries (B = 6), each searched from its first, `cmp(given, held)`.
  The owner is `tree_of_nodes` (src/lower/std_types/map.rs). A quiet and
  total order keeps `$ordMap`, a sorted array, now halving only, which no
  one sees (ADR 0262, none).
- **Its operations are std's, node for node:**
  - `insert_recursing`'s `splitpoint` splits, and a new root;
  - `remove_kv_tracking`: a leaf's steal or merge, an internal key swapped
    with its predecessor, `fix_node_and_affected_ancestors`, and the
    emptied root popped;
  - `split_off`'s per-level `move_suffix` and border fixes;
  - `range`'s bifurcation search, its bounds checked first by `==` and `>`
    as std's are;
  - `extract_if`'s `lower_bound` and its lazy `<=`/`<` end check;
  - `append`, 1.99's `merge`: a cursor from `lower_bound`, `cmp(held,
    given)`, an equal key's entry taken out and put back. `$merging` drops
    as std's frames do: the held value, then the given key, the first key at
    the end; as a held value's drop panics, its key then the given key, the
    given value leaked by std's return slot (also for B-trees not of nodes,
    whose `append` of values with a destructor is no longer refused);
  - a bulk build, `$treeFromSorted(tree, items, TOrd, kind, size, drop,
    dropKey)`: entries sorted by std's stable sort (#925; a map's by `cmp`,
    a set's by `<`), deduplicated by `!=` as `DedupSortedIter` does, then
    `bulk_push` and `fix_right_border_of_plentiful`. `PartialOrd` and
    `PartialEq` are the `Ord` dictionary's `PartialOrd()` and its
    `PartialEq()`, which every `Ord` dictionary holds since #944 (ADR 0057's
    amendment), so `TPartialEq` and `TPartialOrd`, given before for a std
    type's of the crate's, `(K, u32)`, are gone. As the
    sort panics, each entry is dropped in its place; as `!=` panics, the one
    asked, then those after the next, then the next; what's pushed leaks,
    as std's root does. Plain `$treeFrom` keeps its shape for other keys.
    The user chose a helper of its own over extending `$treeFrom`'s arguments;
  - `clone_subtree`: the clone in its source's shape, `ofSorted(made,
    source)`, and as a clone panics, each node's frame drops what it made,
    a later edge's panic first dropping the key and value made before it,
    the value first (`$madeEach(.., dropKey, dropValue)`);
  - `first`, `last` and `pop_*`, and `retain` and `iter`, which ask nothing.
- **One search per entry:** `$entry(m, k)` keeps what it found, `[m, key,
  found]`. `or_insert`, `or_insert_with` and `or_default` of a tree that may
  be nodes are `$entryOrInsert($entry(m, k), v, handle, drop)` and
  `$entryOrInsertWith(..)`, which write it without searching again, and
  make the entry before the value, as Rust does. `$mutGet`, `$someGet` and
  `extend` of a map found by `cmp` search once.
- **Generic code:** a B-tree of a type parameter's keys is its caller's
  class, which may be nodes (`tree_of_nodes`, as `order_seen` says a type
  parameter's order may be seen). Its calls are given what
  nodes ask: `$treeRange`'s `eq` and `partialCmp`, the entry forms,
  `ofSorted`'s source, and `$treeFromSorted`'s `TStableSort` and `TSize` of
  its entries, a slot's or a built slot's (`tree_built_entries`).

```js
const m = new ($ordTreeMap(keyOrd_cmp))();
const orInsert = $entryOrInsert($entry(m, key(random(span))), 0, true);
return $treeFromSorted(new TTreeSet(), $iter(xs), TOrd, TStableSort, TSize);
```

**Proof.** The corpus runs these against native Rust:
- `btree_nodes_seen` (wasm32): maps and sets of 0 to 300 keys of a counting
  order, and of one that isn't total, each operation's calls, trace and
  order;
- `btree_nodes_unwinds`: each operation panicking at its first forty and
  last ten comparisons;
- `btree_nodes_generic`;
- `btree_append_drops`: quiet, numbers and seen keys, with a value's drop
  panicking;
- `btree_clone_nodes_unwinds`.

A sweep of 360 seeds of the first two matched native Rust with no
difference: 47,520 operations over 27.3 million comparisons, and 243,552
unwind runs, 183,842 of them panicking at a chosen comparison. The sweep
found `merge`'s first key dropped at its end. #911's 20 `entry`
case-builds pass. `test/lowering-std-unwinds.test.ts` pins both build forms.

**Left, and filed:**
- a set's `union`, `intersection` and the rest are std's lazy merges (#943);
- generic `==` and `<` under `T: Ord` ask `cmp` (#944, ADR 0057). A
  dictionary without `PartialOrd()` still dedups by `cmp`;
- `extract_if`'s owned bounds aren't dropped (#945);
- a panic inside `split_off` leaves std's nodes unfixed, which native Rust
  traps on, so it isn't compared (#947).

## Amendment: a set's algebra is std's lazy iterators, where its order can be seen (#943)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. An `Ord` that
prints, counts or panics saw other calls, at other times.

A B-tree set's `union`, `intersection`, `difference` and
`symmetric_difference` were its items sorted by JS's `sort` and searched
by `has`, all at once (`$setAlgebra`); `is_subset`, `is_superset` and
`is_disjoint` searched each item. std's are lazy iterators
(`alloc/src/collections/btree/set.rs`): `union` and `symmetric_difference`
step a `MergeIterInner`, asking `cmp(a, b)` of each pair of heads and
holding back the greater; `intersection` and `difference` ask their ends'
`(self_min.cmp(other_max), self_max.cmp(other_min))` where they're made,
then answer from an end, stitch the two walks together, or search the
larger for each of the smaller's items where it's 16 times the size
(`ITER_PERFORMANCE_TIPPING_SIZE_DIFF`); `is_subset` asks its ends, then
searches or walks both, `other1.cmp(self1)`; `is_disjoint` is
`intersection(..).next().is_none()`. Each step is asked as the consumer
asks it.

**Of items std's own nodes hold** (`tree_of_nodes`: an order that can be
seen or isn't total, #926), each is std's, step for step
(`$treeSetAlgebra`, `$treeIsSubset`, `$treeIsDisjoint`, `ord_tree.js`):
a `$Steps` whose `next` takes one of std's steps, its size hint std's.
Stepping a set's items asks nothing, so each reads them as an array;
`contains` is the set's own search, `has`. `$treeSetAlgebra` is one of
`ITERATOR_HELPERS`, a JS iterator already. Of a quiet and total order,
which no one sees asked, they're as they were, `$setAlgebra`'s array (no
difference). No corpus snapshot changed. Corpus case
`btree_set_algebra_seen.rs`: sets of 0 to 300 items of an order that
counts and traces its calls, each operation of each pair of a large, a
smaller, a much smaller, a subset and two ends, stepped with the calls by
each item, and taken only so far; against native Rust in both builds.
Mutations `set-algebra-tipping-halved`, `set-merge-keeps-the-less`,
`set-subset-walk-asked-the-other-way`, `set-difference-stitch-steps-both`
(`runtime/ord_tree.ts`), `tree-set-algebra-unseen` and `tree-subset-unseen`
(`lower/std_types/map.ts`). A set's operators, `&a | &b`, are the next
issue: they made a plain `Set` of the items, which finds an object by
identity.

## Amendment: a set's operator makes a set of its own class (#961)

Case ([0262](0262-when-rust-and-js-disagree.md)): R. `contains` was
`false` where Rust's is `true`, and `insert` added an item the set held.

`&a | &b`, `&`, `-` and `^` of sets, a `BTreeSet`'s and a `HashSet`'s,
were `new Set($setAlgebra(..))`: a plain JS `Set`, which finds an item by
identity. Of items that are objects, a derived `Ord` or `Hash` struct's or
a tuple's, `u.contains(&P(2, 2))` was `false` and `u.insert(P(1, 1))` held
two. std's operator is a set of the receiver's type made of
`self.union(rhs).cloned()`, `from_sorted_iter` of a `BTreeSet`'s, a
`collect()` of a `HashSet`'s.

**The operator makes a set of its receiver's class**, `map_class`'s,
`new $BTreeSet(..)` or `new $KeySet(..)`, of the algebra's items each
cloned (`clone_arg`, `.map(clone)` where a clone isn't the value itself),
which finds each by its order or its hash. **Of items std's nodes hold**
(`tree_of_nodes`), it's std's build: `$treeOfSorted(new Class(), items,
TOrd)`, its items `$treeSetAlgebra`'s lazy merge, each cloned as it's
pulled, those `!=` keeps of each two, `DedupSortedIter`'s, pulled as the
bulk build asks, so the merge's `cmp`, the clones and the `!=` interleave
as std's do; then bulk-pushed (`pushSorted`, now of an array or of a JS
iterator, `$sortedSteps`). The `TOrd`, `TPartialEq` and `TPartialOrd` a
bulk build is given are one function's, `tree_build_dictionaries`,
`made_of`'s and the operator's. `set_operators.js` reads `new
$BTreeSet(..)` and `new $KeySet(..)` where it read `new Set(..)`. Corpus
cases `set_operators_find_by_value.rs`, of a `BTreeSet` and a `HashSet`
of a derived struct's and of tuples, and `btree_set_operators_seen.rs`,
of an order and a clone that count their calls; both builds. Mutations
`set-operator-a-plain-set`, `set-operator-seen-built-unseen`,
`set-operator-items-uncloned` (`lower/std_types/map.ts`),
`tree-of-sorted-keeps-all`, `sorted-steps-pulled-at-once`
(`runtime/ord_tree.ts`); `set-operator-items`,
`tree-from-sorted-dictionaries-ungiven` and `tree-bulk-ne-panic-leaks-rest`
name their lines as they now read.

## Amendment: a library without `alloc` has no B-tree (#983)

`tree_built_entries` asked whether what a generic call makes is a B-tree by
looking up std's `BTreeMap` and `BTreeSet`. A `#![no_std]` library that
loads only `core`, such as powerfmt (of time) or yoke (of url), has neither,
so rust-js crashed compiling it. Now it asks `is_std_def` of the made
type's own item, which answers no where the crate never loaded `alloc`.
The other std items those facts look up (`Ord`, `FromIterator`, `From`,
`Into`, `Iterator`) are `core`'s, which every crate has. Held by the pair
case `no_std_without_alloc` (`test/crates.test.ts`) and the mutation
`tree-built-entries-tree-expected`.
