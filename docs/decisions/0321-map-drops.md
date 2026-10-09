# 0321. A map drops its values as Rust's does

Status: Accepted. Extends [0059](0059-hashmap.md) and
[0098](0098-destructors.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)), but for a key with a
destructor, D: refused.

## Context

A map or a set of values with a destructor was refused: `a std type
holding a value with a destructor`. Since ADR 0320 a counted `Rc` is such
a value, so a `HashMap<&str, Rc<Node>>` of a crate that reads counts was
refused too.

## Decision

**A map drops its values when it's dropped, a `BTreeMap` in its keys'
order, and what `insert` replaces and `remove` takes are the caller's to
drop, as Rust's are.**

```js
const old = $insert(sorted, 2, ["b2"]); // the caller's, dropped as its own
for (const [, value] of $sortedEntries(sorted, $cmp)) {
  noisyDrop_drop(value);
}
for (const item of handles.values()) {
  $rcDrop(item);
}
```

- **A `HashMap`'s values drop in the JS map's order**, which is the order
  they went in. Rust's is its hash order, which no program can count on.
- **What keeps or gives back its values is taken**: `insert`, `remove`,
  `get` and `get_mut`, `m[k]`, `contains_key`, the iterators, `entry`,
  `or_insert_with` and `or_default`, which make a value only to insert it,
  `len` and `is_empty`. Every other function of such a map stays refused,
  as ADR 0098 refuses what might drop a value JS wouldn't: `clear`,
  `retain`, `or_insert`, which drops its value where the key's there, and a
  map made of pairs, `collect()` or `from`, which drops the value a repeated
  key replaces.
- **A key with a destructor is refused**: `insert` of a key that's there
  keeps the old key and drops the new one, which a JS `Map` doesn't. So is
  a set of them.

## Why

- **It's the same program**: each value drops once, where Rust drops it.
- **It's tested**: a corpus case runs `BTreeMap` and `HashMap` values with
  a `Drop`, replaced, removed, read, inserted by `or_insert_with` and
  dropped, and counted `Rc`s removed and dropped, against native Rust; two
  cases pin the refusals of a map made of pairs and of a key with a
  destructor. Mutations drop no values, drop a `BTreeMap`'s unsorted,
  take a key with a destructor, and refuse `remove`.
