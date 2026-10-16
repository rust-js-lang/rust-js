# 0423. hashbrown's maps are std's

Status: Accepted. Extends [0059](0059-hashmap.md), whose `HashMap` is a
JS `Map`, and [0415](0415-a-crate-of-crates-io-in-an-app.md), whose crates
rust-js reads as rustc builds them are serde's.

Case: A ([0262](0262-when-rust-and-js-disagree.md)) for what a map holds
and gives; B for the order it gives its entries in, a hash's in Rust and
the order they were added in JS, as std's `HashMap` (ADR 0059).

## Context

hashbrown is the hash map std's `HashMap` is made of, and many crates use
it directly, `hashbrown::HashMap` and `HashSet`. Compiled as it's written,
it's a SwissTable: control bytes read eight at a time as a `u64`, items
reached by stepping a pointer back from them, a dangling pointer for a
zero-sized item, and std's `Hasher`s. rust-js refused it at 125 places,
and what it would write of them is a hash table engine, not the JS a
person writes, who has `Map`.

## Decision

**hashbrown is read as rustc builds it, never compiled by rust-js, and its
`HashMap`, `HashSet` and map `Entry`, `OccupiedEntry` and `VacantEntry` are
std's: what std's are in JS, a `Map` and a `Set`, their methods std's.**

```rust
let mut counts: hashbrown::HashMap<String, u32> = hashbrown::HashMap::new();
*counts.entry(word.to_string()).or_insert(0) += 1;
```

```js
const counts = new Map();
const current = $orInsert(counts, word, 0);
counts.set(word, (current + 1) >>> 0);
```

- **Which crates rust-js reads as rustc builds them is
  `cargo::READ_AS_RUSTC_BUILDS_THEM`'s**: serde's, and hashbrown, with
  foldhash and allocator-api2, which it alone uses. Such a crate is
  rustc's wherever it is, a registry's or the app's own by its path.
- **Which of hashbrown's items are std's is `recognition::hashbrown_path`'s
  to say**, by its own path, `hashbrown::map::HashMap`: `is_std_def` of
  `HashMap`, `HashSet` and `HashMapEntry` asks it, and the map entries'
  `map_entry_item`.
- **What std's maps haven't, hashbrown's own, is refused where it's
  called**, `allocator()`, `get_key_value_mut`: rustc built the crate, so
  there's no JS of it.

## Why

- **It's the program Rust runs**: `test/hashbrown.test.ts` runs
  `scripts/crate-corpus/hashbrown.rs`, counting with an entry, getting,
  changing and removing, a map's iterator held in a struct, and a set's
  inserts and intersection, against native hashbrown; and refuses a method
  std's maps haven't. `test/registry-crates.test.ts` holds that a crate
  named `hashbrown` isn't compiled, a registry's or the app's own.
- **It's what a person writes**: a `Map`, as std's `HashMap` already is.

## Consequences

- hashbrown's `HashTable` and raw-table APIs, and its own methods std's
  maps haven't, are refused, where they're called. indexmap uses
  `HashTable`.
- A map's iterator's `len()` or `size_hint()`, held and stepped, is
  refused, as std's is.
