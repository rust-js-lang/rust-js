# 0466. A generic map's class is its caller's, a type fact of its key

Status: Accepted; extended by [1095](1095-a-std-traits-generic-method-carries-what-its-declaration-says.md)
(a std trait's generic method's dictionary carries what its declaration
says). Extends [0145](0145-type-facts.md) to a map's class, and
to a trait's generic methods and a type parameter's std associated types;
extends [0121](0121-value-keys.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same map, keyed
as its key's type says, wherever it's made.

## Context

A `HashMap<K, V>` is a JS `Map`, or a `$KeyMap` where its key is found by
value, a struct's or a tuple's (ADR 0121): its class is its key type's. In
generic code, `K` is its caller's, so the class can't be chosen where the
map is made, and a map of a type parameter's keys was refused. itertools
counts and groups its items into maps of `K`s and of `Self::Item`s, in a
trait's defaults and generic methods.

The two classes are used alike, `m.get(k)`, `m.set(k, v)`, `for of`: only
making one tells them apart.

## Decision

**A map or a set of a type parameter's keys, or of a std associated type of
one, `Self::Item`, is of the class its caller's type says, given as a type
fact (ADR 0145), `KMap` or `KSet`.**

```js
function group(iter, KHash, KPartialEq, KMap) {
  const map = new KMap();
  ...
}
group(pairs, ..., Map);
group(points, ..., $KeyMap);
```

- **Which classes are `TypeFact::Map` and `TypeFact::Set`**: a caller's of a
  known type, its `map_class`; of its own type parameter, what it's given.
- **A function asks one where a std call makes a map of its key**,
  `HashMap::new()`, `collect()`, `default()`, and where it gives its own
  slot to a function that asks one, instantiated: a callee's `Self::Item`
  given `self.map(f)` is the caller's `K`.
- **A fact's slot is a drop's (`drop_slot`)**: a type parameter, or a std
  associated type of one, after them (ADR 0421), named as its drop is,
  `SelfItemMap`.
- **A trait's generic method asks what its default, and its impls, ask of
  its own type parameters**, which a caller through its dictionary gives
  after its dictionaries, and a copied default and an entry pass on, as a
  method's own drops are (ADR 0163). So `type_name::<U>()` of a trait
  method's own `U` is given too, which was refused.
- **A copied default resolves and normalizes in its impl's terms** where its
  impl's bounds and its own hold, `Map<T, F>: Counting` of a blanket impl's,
  and is given the drop of `Self::Item` its impl has of `T::Item`, which it
  read of a supertrait's entry that isn't.

## Why

- **It's the map Rust makes**: `generic_maps` groups, dedups and counts by a
  function's `K`, a trait method's own and its default's `Self::Item`, of
  strings and of structs, `Map`s and `$KeyMap`s, with native Rust, as an app
  and as a library; `own_type_facts` names a trait method's own type
  parameter through its dictionary.
- **One class for one type**: a map of `String`s is a `Map` wherever it's
  made, as concrete code expects, never a `$KeyMap` of generic code's.

## Consequences

- **A map keyed by `Self::Item` in a library's default is refused**, as a
  consumer may key it by a value with a destructor, which a map doesn't
  drop yet (ADR 0321).
- **A `BTreeMap` of a type parameter's keys is refused still**: its order is
  its key's too.

## Amendment: a B-tree's class

A map or a set of a type parameter's keys has its class from its caller,
but only a `HashMap`'s and a `HashSet`'s were asked: a `BTreeMap<K, V>`
or `BTreeSet<T>` of a `K: Ord` was refused (#165). A B-tree's is asked
too, its order its key's `Ord` dictionary's, as a sorted map's always
is. Held by `test/corpus/generic_btree_keys.rs`. A crate trait's
associated type as a key, `HashMap<T::Id, T>`, is still refused.

## Amendment: a B-tree's own fact

A generic B-tree was given its caller's `Map` or `Set` fact, a hashed
map's class: a `$KeyMap` of keys whose `Ord` is the crate's own, which
finds a key by its fields, not by its `cmp`. A B-tree asks its own fact,
`TreeMap` or `TreeSet` (`TypeFact::of_map`), whose class may be an
`$ordMap(cmp)` ([ADR 0404](0404-a-b-tree-keyed-by-value.md) amended).

## Amendment: a `&T` key's class is `T`'s (#313)

A map or a set keyed by a reference to a type parameter's value,
`HashSet<&T>` of `a.iter().collect()`, was refused, "values of type `&T`":
the check of what rust-js represents, the question asked of the caller
(`map_of_param`) and the class the map is made of (`map_class`) each took
a type parameter's key only bare. A shared reference is its value (ADR
0023), so **a `&K` key's class is its `K`'s**, `recognition::class_key`'s
answer, which the three ask: `HashSet<&T>` is `new TSet(..)` of the class
its caller gives for `T`, `$KeySet` of a tuple's. TheAlgorithms/rust's
`have_same_elements`, which every sort's tests compare with, is this.
Held by the corpus case `map_keyed_by_a_params_ref` (a count map, a
B-tree set, `contains` and `==` of two, of tuples and strings) and the
mutations `class-key-unpeeled`, `map-class-of-the-ref` and
`generic-map-refused`; `map-of-param-unsupported`, the same mutation as
`generic-map-refused`, is gone.

A map made of what it's collected from is `new C(items)`, of a caller's
class `C` too; an array's `$iter` was dropped from it, `new Set(xs)`, by
tidy only where `C` was named `Set` or `Map`, so `HashSet<&T>`'s read
`new TSet($iter(xs))` and a tuple set's `new $KeySet($iter(xs))`. **Where
it's made (`made_of`), a class's constructor, which steps what it's given
once, in order, is given the array itself**, whatever the class:
`new TSet(xs)`, `new $KeySet([[1, 2], ..])`; tidy's rule of the names is
gone, its mutation `map-made-of-iter` now. The corpus's `$KeySet`s,
`$PathSet`s and `$CowSet`s read so too.

## Amendment: a map std makes, called or given, or behind the `&mut` it gives (#314)

The caller was asked for a map's class where a std call's own type was a
map of a type parameter's keys, `BTreeMap::new()`, `collect()`. So an
entry's `or_default()`, which gives a `&mut` to the default it made, and a
constructor given as a value, `map(BTreeMap::from)`, made a map whose class
nothing asked for, refused: TheAlgorithms/rust's graph tests,
`graph.entry(v1).or_default().insert(v2, c)` of tuple vertices. **Each std
function, called or given, asks for the class of what it gives, or of what
a `&mut` it gives points at**, which it may have made; a shared `&` it
gives, `&graph[&v]`, it didn't. Held by the corpus case
`generic_map_made_by_entry` and the mutations `map-made-behind-mut-unasked`
and `map-made-by-a-std-fn-unasked`.


## Amendment: a key made of a type parameter is refused (#401)

`xs.into_iter().map(|a| Key { a }).collect::<HashSet<_>>()` in generic
code, its key `Key<T>`, printed `new Set(..)`, a JS `Set`, which finds an
object by itself: of `vec![1, 1]` it held two where Rust holds one, a
wrong answer. So did a `(T, i32)`'s and an `Option<T>`'s of a struct `T`.
What class finds such a key depends on its parameter, a `$KeySet` of
equal ones by value, or a `$keySet` of an object equal only to itself
(ADR 1015), and what the caller gives is `T`'s class alone, never one of
a type made of it. **A map or a set keyed by a type made of a type
parameter, or of an associated type of one (`known_by_bounds`), is
refused in generic code** (`map_class`), where its key's type is written
and where it's only inferred alike; a `T` itself is its caller's, as
before.

Case D: no class rust-js has finds it as Rust does. Listed in
`docs/guide/limitations.md`, whose example `test/limitations.test.ts`
compiles, and held by the mutation `generic-made-key-classed` (`std_types/map.ts`).

## Amendment: a tuple's class, composed of its parts' where it's made (2026-11-11, #1346)

Case ([0262](0262-when-rust-and-js-disagree.md)): Rust's behavior was
refused, so it's kept now.

A map or a set keyed by a tuple or an array made of a type parameter,
`(usize, T)`, was refused in generic code, "made of a type parameter": no
class given here says how one is found. itertools stopped there, at
`self.enumerate().minmax_by_key(..)` in an `Itertools` default, which calls
the blanket impl's copy of another default at `Enumerate<Self>`. Asking a
caller the tuple's class, as a type built of its slots, doesn't settle:
the copy at `Enumerate<Self>` asks it again of `Enumerate<Enumerate<T>>`,
and so on, polymorphic recursion through a blanket impl. Measured with
copies asking built-type facts, built asks went 5, 32, 148, 509, 1,802,
6,573, 24,324 and 90,517 over rounds 0 to 7, to 6 GB.

A tuple's class is a function of its parts', as std's derived `Hash`,
`PartialEq` and `Ord` of a tuple ask each part's in turn. So it's composed
where the map is made, `$keyClassOf(set, sorted, [part classes])`, each
part's given or known here, and a caller is asked its own slots' classes
alone, never the tuple's (the user's decision through m6):

- **The asks.** `map_of_param` gives a tuple's or a known-length array's
  parts of type parameters, each asked as a slot, and `held_facts`, the one
  owner of a type's fact as its parts' (ADR 0975), holds a tuple's class
  as its parts' classes wherever a callee asks it of what it's given.
- **The class.** `map_class` composes `$keyClassOf` of each part's class
  (`key_class_parts`). Each class maker says how it finds its keys, a
  static `way`: by value, by what `parts` keeps a key under (`$keyMap`,
  `$PathMap`, `$CowMap`), by its own `Hash` and `Eq` (`$hashedMap`); a
  B-tree by std's order, its own `Ord` or one that can be seen. The tuple
  is hashed where any part is, hashing each in its order and comparing
  each in its order, short of the first unequal; kept under its parts'
  where any part is; else by value. Sorted, by its parts' `cmp`s in turn,
  std's nodes where any part's order can be seen, its own `Ord`'s where
  any part's is. `$keyClassUnsorted` and `$keyClassSorted` hold the rules,
  held to `test/key-class-ways.txt` by `test/runtime-package.test.ts`.
- **What's still refused.** A struct key made of a type parameter,
  `Key<T>`, whose `Hash` and `Eq` may be its own: #1360.

On itertools, asks settle at round 2: slot asks 85, 215 and 257, built
asks 5, 20 and 20, carried 2. itertools 0.14.0 compiles; its probe stops
at #1361. `test/corpus/map_keyed_by_a_tuple_of_a_type_parameter.rs` (a
blanket impl's default counting `self.enumerate()`, keys found by value,
by their own `Hash` and `Eq` and in B-trees by std's order and by their
own `Ord`, and arrays) runs against native Rust in both builds;
`test/diagnostics.test.ts` holds #1360's refusal. Mutations:
`tuple-key-class-asked-whole` (`lower/recognition/specialization.ts`),
`tuple-key-parts-unasked` (`lower/recognition/type_facts.ts`),
`tuple-key-class-uncomposed` (`lower/std_types/map.ts`) and five of
`runtime/key_class.ts`; four refitted.

## Amendment: a key of no type parameter isn't taken apart (#1368)

Since #1346 the classes a caller gives of a map's key were found by taking
the key apart, a tuple into its parts and an array into an item for each
place (`key_class_parts`), whatever the key: of every map the crate makes,
concrete too. `HashMap::<[u8; 536870907], ()>::with_capacity`,
runtime-package's test of room past wasm32's, was 536,870,907 parts, and
its compile held 10.8 GB (8.6 GB of the one function alone), so a
mutation's baseline, given half the machine's 10 GB budget, was stopped as
it began and reported nothing (#1368, two runs in three of m6-lily's). A
key of no type parameter, whose class is made where it's made, has no part
a caller gives: `map_of_param` and `held_facts` take apart only one that
mentions a type parameter (`mentions_param`), no answer changed. The
compile holds about 120 MB; runtime-package's file takes 6 s in each of
twelve baselines at once, where each took 67 s and failed. Held by
`test/runtime-package.test.ts`' "a map keyed by an array of half a billion
bytes compiles within a gigabyte"; mutation `concrete-key-taken-apart`
(`lower/recognition/type_facts.ts`); `tuple-key-parts-unasked` and
`tuple-key-class-asked-whole` refitted. An array key of a type parameter,
`[T; N]`, is still one part an item, its JS `$keyClassOf` of `N` classes,
so a huge `N` there costs as much: left, as no program has met it.

## Amendment: an array key's class is its item's, once (#1371)

#1368 left an array key of a type parameter, `[T; N]`, one part an item:
`key_class_parts` gave `N` copies of `T`, `map_of_param` and `held_facts`
walked each, and the class was `$keyClassOf(set, sorted, [TMap, TMap,
...])`, `N` classes long. A generic function's
`HashMap::<[T; 536870907], ()>::with_capacity(n)` held 4.5 GB of the
compiler's memory within 0.7 s; `[T; 4096]` compiled, its JS 4,096 lines
of `TMap`.

An array's items are of one type, so its class is that type's applied to
each item: std's `Hash` of an array is its slice's, each item in turn
after the length every key of the type shares; its `PartialEq` compares
each in turn, short of the first unequal; its `Ord` orders them
lexicographically. That's a tuple's of parts all of one way. So
`key_class_parts` gives a tuple's parts, or an array's item once
(`KeyParts`), whatever its length; a caller is asked the item's class
once; and the class is `$arrayKeyClassOf(set, sorted, Item)`, which
shares `$keyClassOf`'s composition (`$composedKeyClass`), each part of a
key found by its way, `ways[index(i)]`, a tuple's `i`th, an array's one.
The user chose this (A) over `$keyClassOf` given one class for an array's
items, and over a list kept for small arrays, two forms of one construct.
`[T; 0]` asks its item's class too, harmless, as every key of it is one.
Case ([0262](0262-when-rust-and-js-disagree.md)): none, each result std's.

A generic function's `[T; 536870907]`, made where it's made and given a
callee that asks it of what it's given, compiles in about 120 MB, held by
`test/runtime-package.test.ts`' "a map keyed by an array of half a billion
of a type parameter's items compiles within a gigabyte" (1 GiB cap).
`test/key-class-ways.txt` says an array's way is its item's, each of the
six, held by `$arrayKeyClassOf` itself. The corpus case
`map_keyed_by_an_array_of_a_type_parameter.rs` (a `HashSet<[T; 3]>` of
keys found by value and by their own `Hash` and `Eq`, a `HashMap<[T;
4096], _>`, `BTreeSet<[T; 2]>`s by std's order and by an `Ord` of their
own, a `BTreeMap` of arrays of tuples, and `[T; 0]`) runs against native
Rust in both builds and through its metamorphic rewrites. Mutations
`array-key-item-unasked` and `array-key-an-item-a-part`
(`lower/recognition/type_facts.ts`), `array-key-held-as-its-items`
(`lower/recognition/specialization.ts`), `array-key-class-uncomposed` and
`array-key-class-a-tuple-of-one` (`lower/std_types/map.ts`), and
`array-key-way-its-first-items-alone`, `array-key-way-by-value` and
`key-compared-by-its-classes-parts` (`runtime/key_class.ts`); five
refitted. The class lowering asks `mentions_param`, the one owner of
whether a key is made of a type parameter, where it walked the key itself.
And `map_of_param` no longer asks it: with an array one part, taking a
key of no type parameter apart costs a part a tuple's part, and gives
nothing, so #1368's guard was an answer no input could tell from its
absence; `concrete-key-taken-apart` survived, and went with it. The
#1368 test holds `array-key-an-item-a-part` too.


## Amendment: a default's own where clause's slots are its callers' (2026-11-11, #1363)

A trait default whose own where clause asks more of `Self::Item`,
`fn counts(self) -> HashMap<Self::Item, usize> where Self::Item: Hash +
Eq`, has that item's class as its own slot, `SelfItemMap`, which each of
its callers gives. But where another default, `distinct`, calls it, its
copy in a blanket impl asked the impl too, of `I::Item`, the slot the
default's where clause gives the impl's bounds (ADR 0478's, for drops):
the dictionary became `iTools(IIterator, IItemMap)`, and every caller of
it gave that class, one calling only `ordered` on items with `Ord` and no
`Hash` too, refused, "implementation evidence for `<Backwards as
Hash>`". A copied default no longer asks its impl a fact of a slot its own
where clause names (`type_fact_params`' copies): the dictionary is
`iTools(IIterator)`, and each default reads its own `SelfItemMap`, which
`given_fact_of` finds once the impl's isn't there.

Held by `default_where_clause_facts_its_callers.rs` (`ordered` of items
with no `Hash`, `counts` keyed by a tuple of `Self::Item`, `distinct`
calling it, of words and of a mapped range) against native Rust in both
builds and as a library, and the mutation `default-own-slot-asked-of-impl`.
