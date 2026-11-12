# 0401. A path compares, orders and keys a map by its components

Status: Accepted. Amends [0173](0173-paths-as-text.md) and extends
[0121](0121-value-keys.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a path's text isn't
what Rust compares, so a string's `===` would change results; where the
key a map holds can't be given back, D: a compile error.

## Context

A path is its text (ADR 0173), but Rust's `==` of two compares their
components: `a//b/./c/` is `a/b/c`, `./a` isn't `a`, and `a/b` orders
before `a.b`, which as text it's after. So `==`, ordering, and a path as
a map's key were refused. rustc's `hashmap-path-key.rs` failed on it.

## Decision

**A `Path` or a `PathBuf` compares, orders and keys a map by its
components, as Rust's `Path::components()` has them on Unix: `$pathEq`,
`$pathCmp`, and a map or a set of paths, `$PathMap` or `$PathSet`, a
`$KeyMap` or `$KeySet` keeping each under `$pathKey`.**

```js
console.log(`${$pathEq(a, b)}`);
v.sort($pathCmp);
const sizes = new $PathMap();
sizes.get("src/./main.rs");
```

- **Components**: `/` first if it's absolute, `.` first if it starts with
  one, then each part between separators but an empty one or `.`; `..`
  is kept. `$pathKey` is one string of them, which no other components
  make, and `$pathEq` compares those.
- **Order**: Rust's `Component`'s, the root, then `.`, then `..`, then a
  name by code point, each in turn, a shorter one first.
- **An `OsStr` beside a path compares as a path**, as std's impls make
  it; two `OsStr`s compare as text.
- **A map gives back the key it holds**, as it was put in, which `a//b`
  finds as `a/b`. A `$KeyMap`'s `keyOf` is what a path map changes.
- **A B-tree made of items** keeps the last of equal paths, as std sorts
  them and keeps the last, where `insert` keeps the first: `$treeFrom`.
- **Still refused**: what gives back the key a path map holds for one
  asked with, `get_key_value`, `remove_entry`, a set's `get`, `take` and
  `replace`, an entry's `key()`, which would give the one asked with; and
  the path methods not in ADR 0173, `join` and `parent` among them.

## Why

- **It's the program Rust runs**: the corpus's `path_compare.rs` compares
  and sorts paths with `.`, `..`, roots and doubled separators, an `OsStr`
  beside one, and `path_keys.rs` keys hashed and B-tree maps and sets by
  paths, made by `insert`, `entry`, `collect` and `from`, each with native
  Rust; the rustc test passes. Mutations keep `.` parts, drop the root,
  order components as text, key a map by its text, and keep a B-tree's
  first.

## Amendment: the key it holds, given back

What gives back the key a map holds, refused above, gives it: `$storedKey`,
the one it holds equal to the one asked with ([ADR 0404](0404-a-b-tree-keyed-by-value.md)
amended). Held by `test/corpus/held_keys.rs`.

## Amendment (2026-11-12, #1467): inside what holds it too, one owner

Case ([0262](0262-when-rust-and-js-disagree.md)): results changed, so
Rust's behavior is kept. A path alone compared by `$pathEq`, but a tuple
holding one by structural `$eq`, its text: `(PathBuf::from("a/b"), 1) ==
(PathBuf::from("a//b"), 1)` was `false` where Rust's is `true` (P0), and a
hash set of such keys kept both once #1447 hashed them. `custom_eq` said a
std type with no type arguments compares as its value. **One owner says
it, `compares_by_components`** (recognition/types.rs): `==` (`eq_value`)
and order (`cmp_value`) ask it for `$pathEq` and `$pathCmp`, `custom_eq`
counts it custom, so a tuple, an array, a derived struct, an `Option` and a
`Vec` compare part by part and reach them, and `Hash` asks it for
`$hashPath`; a table's keys, by `compares_by_value` and `orders_by_value`,
already never took a path for its text. An `OsString`'s `==` and order are
its text's, which the `$cmp` of text orders by UTF-8 bytes, as std's.

Held by corpus case `paths_compare_by_components_inside.rs` against native
Rust in both builds; mutation `path-in-tuple-compared-as-text`.
