# 0466. A generic map's class is its caller's, a type fact of its key

Status: Accepted. Extends [0145](0145-type-facts.md) to a map's class, and
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
