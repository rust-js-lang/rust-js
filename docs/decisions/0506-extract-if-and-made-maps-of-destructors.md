# 0506. `extract_if`, and a map made of items, of values with a destructor

Status: Accepted. Extends [0344](0344-extract-if.md) and
[0098](0098-destructors.md); amends [0321](0321-map-drops.md), whose
refusal of a map made of items with a destructor it reverses.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped and
when would change, so Rust's is kept, paid for in output.

## Context

`v.extract_if(range, f)` of a `Vec`'s or a `LinkedList`'s values with a
destructor was refused, and so was a map's or a set's: `extract_if` wasn't
among the calls known to keep what they take out, and its iterator's drop,
std's own `Drop`, was taken to drop the items it holds.

`HashMap::from([(k, v)])`, and `collect()` into a map or a set, of keys or
values with a destructor was refused (ADR 0321): `new Map(pairs)` replaces
the value of a repeated key without dropping it. A port writes both; they
are std's.

## Decision

**`extract_if` gives what it takes out to whoever steps it, each dropped
where what it's given to drops it; its iterator's drop is its test's, and
what it hasn't reached stays in the collection, dropped with it.**
`extract_if_test`, recognition's, names the test an `ExtractIf` holds (`F`,
before its allocator); the drop question asks it alone. A test that holds a
value with a destructor stays refused, as any function kept by what it's
given to is.

**A map or a set made of items drops what an equal one replaces, as std
does.** `made_of`, the one place `collect()` and `from()` make a map, gives
the keys' and the values' drops:

- **A hash map or set**: each inserted as `insert` does, the key it's given
  dropped where an equal one's there, and the value it replaces, as it goes:
  `$extendMap(new Map(), items, false, drop, dropKey)`, `extend`'s helper,
  which now gives back its map.
- **A B-tree**: std collects what it's made of, sorts it stably by key and
  keeps the last of equal ones, dropping the earlier, key then value, in
  its keys' order: `$treeFrom(new Map(), items, false, cmp, drop, dropKey)`,
  given the keys' `cmp` always, as it now sorts.

A map whose keys and values drop nothing is `new Map(items)`, as before.

## Why

- **It's the same program**: `extract_if_drops` runs a `Vec`'s
  `extract_if` collected and stopped early, a list's counted, a B-tree's
  stepped by `for` and a hash map's counted; `map_collect_drops` makes
  hash and B-tree maps and sets with repeated keys from arrays and from
  `collect()` of a printing iterator; both against native Rust.
- **One place**: both `collect()` and `from()` go through `made_of`, so a
  map made of items has one answer to what it drops.

## Consequences

- A B-tree made of values with a destructor is `$treeFrom(...)` even where
  its keys can't repeat, `(1..7).map(..).collect()`'s: whether they can
  isn't known to the compiler.
- `$treeFrom` takes the keys' `cmp`, so a B-tree of paths or `Cow`s made of
  items (ADRs 0401, 0402) is given it too.
- Mutations sort none, drop no earlier key or value, give back no map,
  drop nothing an equal one replaces, refuse `extract_if` and a map made of
  items, and take an `ExtractIf` to drop what it holds.
