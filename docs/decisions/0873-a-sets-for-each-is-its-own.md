# 0873. A set's `for_each` is the set's own `forEach`

Status: Accepted. Extends [0055](0055-iterator.md)'s eager chains,
an array's own methods, to a set's.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: a `Set`'s `forEach` gives each item once, in the order its
iterator does, which `Array.from(set)` lists; Rust's borrow keeps the set
unchanged while it's stepped, so no item is added or removed meanwhile.

## Context

React's ReactTaint gives each pending request the value it cleans up:

```js
TaintRegistryPendingRequests.forEach(function (requestQueue) {
  requestQueue.push(entryValue);
  entry.count++;
});
```

Its port's `TAINT_REGISTRY_PENDING_REQUESTS` is a `HashSet<&'static
RequestCleanupQueue>`, a JS `Set` by ADR 0864, and Rust steps it as
`pending.borrow().iter().for_each(|request_queue| ..)`. A set's items are
`Array.from(set)` (`map_items`), so the eager chain was an array's:

```js
Array.from(pendingRequests).forEach((requestQueue) => { .. });
```

## Decision

**A set's items, `Array.from(set)` of a `HashSet`'s `Set` or a class of
it, `$KeySet`, are known to be a set's (`js::Known::SetItems`, which
`map_items` says), and their `forEach` of a callback that reads the item
alone, an arrow of one parameter or none, is the set's own**
(`eager::set_of`):

```rust
tags.iter().for_each(|tag| println!("{tag}"));
```

```js
tags.forEach((tag) => {
  console.log(`${tag}`);
});
```

- **Only of a callback of one parameter**: a set's `forEach` gives the
  item, then the item again and the set, where an array's gives its index
  and the array; a function given as it is, `forEach(g)`, may read them,
  so it stays the array's.
- **Only of the set's items, stepped as they are**: a stage before it,
  `map`, `filter`, `enumerate`, makes an array of its own, whose
  `forEach` is the array's. A map's `values()` are no set's:
  `Array.from(map.values()).forEach(..)` as before.
- **A B-tree's set is in its keys' order**, `$sortedKeys(..)`, no `Set`'s
  own order: never known so.

## Why

- **Upstream's JS**, `set.forEach(..)`, which a person writes, not a copy
  of the set made to step it.
- **The same items, in the same order**: `Set.prototype.forEach` visits
  what's there in insertion order, as its iterator does, and so does
  `$KeySet`'s, whose `forEach` steps the items it keeps.

## Alternatives

- **`for (const x of set)`**: a loop where Rust wrote `for_each`, a
  statement where upstream calls the set's method.
- **Recognize `Array.from(x).forEach` wherever it's written**: a
  `NodeList`'s `Array.from` of a JS iterator, or a map's values, aren't a
  set's; the JS shape doesn't say what made it, which the fact does.

## What holds it

- Corpus `set_for_each` against native Rust: sets of numbers, text,
  tuples (`$KeySet`), objects by themselves through a `RefCell`, and a
  callback of nothing; a map's values, still the array's.
- The guide's Collections example (types.md), `tags.forEach(..)`.
- Mutations `set-for-each-listed`, `set-for-each-of-any-items`
  (`lower/iterators/eager.ts`) and `set-items-unknown`
  (`lower/std_types/map.ts`).
