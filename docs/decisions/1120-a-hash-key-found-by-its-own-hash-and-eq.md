# 1120. A hash map's key is found by its own `Hash` and `Eq`

Status: Accepted. Supersedes what remained of
[0168](0168-user-hash.md), that a map never calls `hash`, and amends
[0463](0463-hashing-is-stds.md), which kept it. Extends
[0121](0121-value-keys.md) and [0404](0404-a-b-tree-keyed-by-value.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results changed, so
Rust's behavior is kept. A key whose `Eq` is its own was refused, and a
key whose `hash` panics or prints ran without calling it, where std's
table calls it on each search.

## Context

A `HashMap` or `HashSet` was a JS `Map` of keys JS finds by itself, or a
`$KeyMap` of keys found by their value's string (ADR 0121). A key whose
`PartialEq` is the crate's own was refused (#1271), a case-insensitive
word, a user equal by its id:

```rust
impl PartialEq for Word {
    fn eq(&self, other: &Word) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Hash for Word {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_ascii_lowercase().hash(state);
    }
}
```

That's ordinary Rust, and neither JS identity nor every field finds such
a key: `Word("Apple")` must find the `Word("apple")` the map holds.

A key whose `Eq` is derived but whose `Hash` is its own was a `$KeyMap`,
which never called `hash` (ADR 0168): Rust requires equal keys to hash
alike, so which keys a map holds is `Eq`'s alone. But std's table calls
`hash` on every search. A `hash` that panics panicked in Rust and ran on
in JS; one that prints printed in Rust alone.

## Decision

**A hash map's or set's key whose `==` JS can't tell, or whose `hash`
runs a crate's own code, is found as std's table finds it: by the bytes
its `Hash` writes, then by its `eq`.** The map is a `$hashedMap` of the
key's `Hash` dictionary and its `eq`, a set a `$hashedSet`:

```js
const counts = new ($hashedMap(wordHash, wordPartialEq_eq))();
```

- **One rule, `found_by_hash`** (`std_types/map/keys.rs`): a key JS can't
  find by itself or by its value (`is_key`), or whose `hash` runs code
  (`hash_runs_code`, `std_types/hash.rs`). `map_class` asks it, and so
  does `finds_others`, so a set's `get` and `take` give back the key the
  set holds. `()` stays JS's `undefined`.
- **`hash_runs_code`** walks what `hash` reaches: std's writes and a
  derived impl's fields run none of a crate's code; a hand-written impl,
  the crate's or a library's, does; std's impl of another type hashes what
  its own `Hash` bounds name, `Reverse<T>`'s `T`, never
  `Discriminant<T>`'s. A type equal only to itself hashes its address
  (ADR 0864).
- **A search calls `hash` once**, writing to a `$KeyHasher`, a `$Hasher`
  that keeps every byte written, so the bytes tell keys apart as every bit
  of a hash does. It's given as a std hasher, `{ value }` with
  `$sipHasher`. Its `finish()` is a `DefaultHasher`'s of the bytes.
- **`eq(key, held)` is asked of each key held under the same bytes**, in
  the order they were put, the given key first as hashbrown's
  `equivalent_key` asks it. Keys whose bytes differ are never compared.
- **`insert` of a key it holds keeps the key it holds**; a set's
  `replace` puts the one given in its place; `retain` takes out what it
  walks to without hashing, as std's does.
- **A search panics before the table changes**, as std's does where
  `hash` or `eq` panics.
- **An entry is one search** (#1362): `m.entry(k)` is `$entry(m, k)`,
  which hashes `k` once, and `or_insert`, `or_insert_with(_key)`,
  `or_default`, `and_modify` and the handle on a number or text they give
  write what it found, `$entryOrInsert` and `$entryOrInsertWith` as a
  B-tree of std's nodes' (#926), never `$orInsert(m, k, v)` and `m.set(k,
  ..)`, three or four searches. `entry_searched_once` (`map_keys.rs`)
  owns which entries are: a B-tree's of std's nodes, and a hash map's of
  keys found by their own `Hash` and `Eq`, generic code's included, whose
  class may be one. `get_disjoint_mut`'s handle is on what its search
  found, and a walk hashes nothing: `values_mut`, `iter_mut` and `retain`
  hold each entry itself (`entryHandles`), and `extract_if` takes out
  what it walks to (`$deleteHeld`), as std's do.

## What std leaves its own

How often a table calls `hash` and `eq` is std's own. hashbrown hashes
every key again as it grows, and compares keys whose hashes differ where
7 bits of their random hashes meet. Its clone copies the table without
hashing. rust-js hashes once a search, a clone's keys too, and never
compares keys whose bytes differ. A program can't count on either, so the
corpus prints what a map holds and gives back, and counts calls only of a
table made `with_capacity` for all it holds, which std's docs say never
grows: there each call's searches are std's, one an entry and none a
walk (`test/corpus/map_entry_hashes_its_key_once.rs`, #1362). The std
net's counted key counts its `hash` alone, each search's one in a table
that never grows, printed and panicking at its 2nd (ADR 1090, amended,
#1362).

## Why

- **It's std's answer**: `test/corpus/keys_found_by_their_own_hash.rs`
  has a case-insensitive word in maps and sets: `entry`, `get`,
  `get_key_value`, `get_mut`, `remove_entry`, `retain`, `extend`,
  `clone`, `==`, `replace`, `take`, the set algebra, `drain` and
  `values_mut`. It also has a user equal by its id, a key every value of
  which hashes alike, a tuple key and a derived key holding a word,
  generic code's map, and a `hash` that panics, of the key itself, a
  derived key's field and an `Option` of one. All of it runs against
  native Rust in both builds. The std net (#1171) runs its counted key's
  cases, `HashMap` and `HashSet`, which it pinned as refused.
- **One class per job**: a key JS finds by itself is a `Map`'s, one found
  by its value a `$KeyMap`'s, one found by its own `Hash` and `Eq` a
  `$hashedMap`'s.

## Consequences

- ADR 0168's keys whose `Hash` is their own are now `$hashedMap`s:
  `user_hash`'s `Version`, `Channel` and `Tagged`. They call `hash`, its
  impl's JS written where the map reaches it.
- **Refusals lifted**: an `Rc` or `Arc` key, a string interner's
  `Rc<str>`, is a `$hashedMap` of what it points at's `Hash` and `==`, its
  clones counted as std's are; `()` is a `Map`'s `undefined`, one key; a
  type equal only to its own address but for its own `ne`, or whose `eq`
  reads more than its address, is found by its own `eq`.
  `test/corpus/keys_of_rc_arc_and_unit.rs` runs them against native Rust in
  both builds.
- **Refused**: a lookup of such a map by what its key's `Borrow` gives,
  another type, `HashSet<Rc<u32>>::contains(&1)`, which std finds by that
  type's `Hash` and `Eq` (`hashed_key_borrowed`, `traits/calls.rs`;
  #1288); a
  crate's own `Borrow` stays refused as before (ADR 0167). And a key of
  std's that rust-js doesn't hash, whose own `Hash` reaches a crate's,
  `HashSet<Reverse<Version>>`: "values of type .. as a hash map's keys,
  whose `Hash` rust-js doesn't write (#1289)". It was a `$KeySet` that
  never called `hash`, a wrong answer where it panics or prints (#1287).
  Since #1289 std's wrappers and enums are hashed as std's impls write
  them (ADR 0463, amended); what's left names #1434 or #1435.
- Mutations: `hashed-key-by-identity`, `hashed-key-own-hash-unseen`,
  `hashed-key-given-back`, `hash-runs-code-of-derived-unwalked`,
  `hash-runs-code-of-any-bound`, `hashed-key-refused`,
  `hashed-key-borrowed-unrefused`, `hashed-keys-eq-unasked`,
  `hashed-keys-removed-still-found`, `hashed-keys-insert-keeps-new-key`,
  `hashed-set-replace-keeps-held`; and of an entry's one search (#1362),
  `hash-entry-searched-again`, `hashed-entry-handles-search`,
  `disjoint-mut-handle-searches`, `extract-if-deletes-by-search`,
  `extract-if-set-deletes-by-search` and `mut-values-searched`.
- **Amended (#1362)**: a hash map's entry of such keys hashed its key three
  or four times, `$orInsert` then `m.set`, where std's hashes it once, and
  a walk's handles searched for each key; the same in generic code. A
  B-tree's `values_mut` of a key whose `Ord` is its own compared its keys
  where std's walk compares none, now its entries' handles too.

## Amendment (2026-11-12, #1288): a lookup by what the key's `Borrow` gives, where the key is that value in JS

Case ([0262](0262-when-rust-and-js-disagree.md)): none for results; a
refusal narrowed. A table whose keys are found by their own `Hash` and
`Eq` hashes a key of their type alone, so a lookup by what the key's
`Borrow` gives, another type, was refused, `HashSet<Rc<u32>>::contains(&1)`.
But std's `Borrow` impls keep `Hash` and `Eq` the same, `hash(k.borrow())
== hash(k)`. Where the key is in JS the very value it borrows as, the
table's own lookup, the key's `hash` and `eq` of that value, is std's.
`borrowed_is_the_key` answers which keys are so (std_types/map_keys.rs):
an `Rc` or an `Arc` nothing counts, its value; a `Box<T>`, its `T`; a
`Vec<T>`, its `[T]`'s array; a `String`, its `str`; a reference, what it's
to. Each is hashed once per lookup, as std hashes it.

Still refused: a counted `Rc`, whose JS is its counts' object, `{ value,
strong }`, not the value looked up (#1433); its message says so, and the guide's
limitation keeps it. The crate's own `Borrow` stays refused as before
(ADR 0160).

Held by corpus case `hashed_keys_looked_up_by_their_borrow.rs` (an `Rc`
of a number, an `Rc` and a `Box` of a struct whose `Hash` counts its
calls, a `Vec` by its slice) against native Rust in both builds, and a
diagnostics row of the counted one. Mutations `borrowed-key-*`.
