# 0402. A `Cow` keys a map by what it borrows

Status: Accepted. Extends [0400](0400-a-cow-compares-as-what-it-borrows.md)
and [0401](0401-a-path-compares-by-its-components.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): std's `Hash` and
`Eq` of a `Cow` deref it, so `Owned("a")` finds the `Borrowed("a")` a
map holds; a map keyed by its object would tell them apart. Where the key
a map holds can't be given back, D: a compile error.

## Context

`HashMap<Cow<'static, str>, usize>` and its B-tree were refused: a `Cow`
is `{ TAG, _0 }` (ADR 0319), which `$KeyMap` would key by its variant
too. rustc's `send_str_hashmap.rs` and `send_str_treemap.rs` failed on it.

## Decision

**A map or a set of `Cow<str>`s, or of `Cow<[T]>`s of what compares by
value, is a `$CowMap` or `$CowSet`: a `$KeyMap` or `$KeySet` keeping each
under what it borrows, `$cowKey`'s, which a `&str` or a `&[T]` asked
with is keyed by too.**

```js
const counts = new $CowMap();
counts.set({ TAG: "Borrowed", _0: "a" }, 1);
counts.get("a");
```

- **What it borrows is never an object like a `Cow`'s**, text or an
  array, so `$cowKey` knows a `Cow` from what it borrows by its shape. A
  `Cow<T>` of another `T`, which may be such an object, stays refused.
- **The key it holds is the first put in**, as `insert` keeps it; a
  B-tree made of items keeps the last of equal ones, as std's does,
  `$treeFrom`'s, as of paths (ADR 0401). One question, `finds_others`,
  answers both, for paths and `Cow`s.
- **Still refused**: what gives back the key a map holds for one asked
  with, as of paths: it would give a `&str` where a `Cow` is.

## Why

- **It's the program Rust runs**: the corpus's `cow_keys.rs` keys hashed
  and B-tree maps and sets by borrowed and owned text, asks with `&str`
  and with `Cow`s, shows which variant each holds, and keys a map by
  `Cow<[i32]>`, each with native Rust; both rustc tests pass.

## Amendment: the key it holds, given back

What gives back the key a map holds, refused above, gives it: `$storedKey`,
the one it holds equal to the one asked with ([ADR 0404](0404-a-b-tree-keyed-by-value.md)
amended). Held by `test/corpus/held_keys.rs`.
