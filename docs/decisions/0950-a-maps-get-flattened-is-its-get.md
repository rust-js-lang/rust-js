# 0950. A JS map's `get` flattened is its `get`

Status: Accepted. Extends [0051](0051-generic-options.md)'s `flatten()`,
`$someValue` of an `Option` of an `Option`, and
[0225](0225-unknown-values.md)'s keyed read of a value that may look like
`None`, `$someGet(map, key)`, to the two together.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Of each key, what the
two helpers gave is what `map.get(key)` gives: `undefined` where the key
isn't there, the value where it is, a `None`'s `null` or `undefined` a
`None` alike.

## Context

React's `getCacheForType` reads a cache's map, `cache.data.get(resourceType)`,
whose values may be `undefined`. Its port, a `js::Map<str, Option<&Unknown>>`,
wrote `data.get(key).flatten()`: `$someValue($someGet(data, key))`, which
boxes a value that looks like `None` to tell a key that's there from one
that isn't, then takes the box off again.

## Decision

**`flatten()` of `$someGet(map, key)` is `map.get(key)`.** `$someGet`
gives `undefined` where `map.has(key)` doesn't, else `$some(map.get(key))`,
and `$someValue` of that is `map.get(key)`'s value; the map's `get` alone
gives each, one read of a JS `Map`, which runs nothing.

## What holds it

- `test/lowering-collections.test.ts`, "a JS map's get flattened is its
  get".
- Mutation: `map-get-flatten-boxed`.
