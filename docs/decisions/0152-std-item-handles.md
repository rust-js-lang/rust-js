# 0152. A `&mut` std hands out to a number or a string is a handle on it

Status: Accepted. Extends [0099](0099-mut-references.md).

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A JS number or string in an array or a `Map` can't be pointed at: there's
no reference to `v[1]`, only `v` and `1`. So a `&mut` std gives to one,
`get_mut`'s or `iter_mut()`'s, was the item itself (ADR 0099), which a
pattern can read but nothing can write through once it's passed on. Each of
these was an error, though they're how Rust changes what a collection holds:

```rust
scores.iter_mut().for_each(|s| *s *= 10);
for n in stock.values_mut() {
    *n += 1;
}
bump(row.last_mut().unwrap());
```

And one compiled to JS that throws: `for (_, n) in &mut m` of a map of
numbers wrote `entry[1].value` of each number.

## Decision

**Where std hands out a `&mut` to a number or a string in a collection, used
as a value, it's a handle on the item:** `{ get value() { .. }, set value(x)
{ .. } }`, which reads and writes its slot, as the handle on a variable
does (ADR 0099). A `&mut i32` is already read and written as `x.value`, so
whatever it's given to works unchanged.

| Rust, of numbers or strings | JS |
|---|---|
| `v.iter_mut()` | `$mutItems(v)`: a handle on each item |
| `m.values_mut()`, `m.iter_mut()` | `$mutValues(m)`, `$mutEntries(m)`: each writes `m.set(key, ..)` |
| `for (k, n) in &mut m` | `for (const item of $mutEntries(m))` |
| `v.get_mut(i)`, `first_mut()`, `last_mut()` | `$mutAt(v, i)`, `$mutAt(v, 0)`, `$mutAt(v, -1)`: a handle, or `undefined` |
| `m.get_mut(&k)` | `$mutGet(m, k)`: a handle, or `undefined` |

- **A B-tree's go in its keys' order,** `$mutEntries(m, $sortedEntries(m,
  $cmp))`, as its other iterators do (ADR 0059).
- **What passes one on keeps it a handle:** `filter`, `enumerate` and
  `collect()` of handles, and `unwrap()` of `last_mut()`, which a function
  taking a `&mut i32` is given as its box.
- **Of objects, nothing changes:** a `&mut` to an object is the object
  (ADR 0025), so `get_mut` of structs is `v[i]`.
- **Where it was before, it stays:** `for x in v.iter_mut()` is an index
  loop, and `if let Some(n) = m.get_mut(&k)` reads and writes the map's
  slot. A map's `get_mut` matched by a pattern still binds the item.
- **Still errors:** a map's `get_mut` bound by a `match` and kept past it.

## Why

- **It's exact:** each handle writes the slot it was made for, so the
  collection changes as Rust's does. The `std_mut_items` corpus case
  compares closures, map loops in hashed and sorted order, kept and
  passed-on handles, and collected ones with native Rust.
- **It's the JS a person writes:** `$mutItems(scores).forEach((s) => {
  s.value *= 10; })`, a getter and a setter where JS has no pointer.

## Consequences

- A handle is an object per item: `iter_mut()` as a value allocates, where
  an index loop doesn't.
- The `&mut m` loop of numbers, which threw, writes the map.
