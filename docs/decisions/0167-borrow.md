# 0167. `Borrow`: a dictionary, the value itself for std's

Status: Accepted. Extends [0049](0049-traits-and-generics.md) and
[0162](0162-generic-as-ref.md).

## Context

`Borrow` is how Rust says "this key can be looked up as that": a map's
`get(q)` takes any `Q` its `K: Borrow<Q>`. Code that looks things up
generically takes the same bound, as the `equivalent` crate, which
indexmap and hashbrown use, does:

```rust
impl<Q: ?Sized + Eq, K: ?Sized + Borrow<Q>> Equivalent<K> for Q {
    fn equivalent(&self, key: &K) -> bool {
        PartialEq::eq(self, key.borrow())
    }
}
```

And a type gives its own, uuid's `impl Borrow<Uuid> for Hyphenated`.
`key.borrow()` of a generic `K` and a type's own `impl Borrow` were errors:
they stopped uuid, deranged and equivalent, so time and indexmap.

## Decision

**`Borrow` is a trait of dictionaries, `{ borrow }`,** as `AsRef` is (ADR
0162): a generic function takes `K`'s, and `key.borrow()` in it is the
dictionary's.

```js
function contains(keys, wanted, KBorrowQ, QPartialEq) {
  return keys.some((key) => QPartialEq.eq(KBorrowQ.borrow(key), wanted));
}

contains(words, "milk", { borrow: (value) => value }, { eq: (a, b) => a === b });
contains(tags, "blue", tagBorrowStr(), { eq: (a, b) => a === b });
```

- **std's is the value itself** where it changes nothing in JS: a value as
  itself, through a reference, a `Box` or an `Rc` (ADR 0023), text as a
  `str`, and a `Vec` or an array as a slice. Where the types are known,
  `s.borrow()` is written as the value, no dictionary. Others, a `Cow`'s
  say, are errors.
- **The crate's is its impl's,** `tagBorrowStr()`.
- **A blanket impl over a `?Sized` type,** `impl<Q: ?Sized> Equivalent<K>
  for Q`, is a dictionary of its impl's own methods: rustc won't resolve
  one there, as a `dyn` might be `Q` too, but in its own dictionary it's
  the impl's.
- **std's functions that would take a value as what its own `Borrow`
  gives are errors:** a map's or a set's `get`, `contains`, `remove` by a
  `Q` of keys whose `Borrow<Q>` is the crate's, and `[S]::join` or
  `concat` of items whose `Borrow<str>` is. std's JS compares and joins
  the value itself, not what it borrows as, so these would find nothing,
  or join the wrong text. A lookup by the key itself is std's, and works.
  A std function that calls the crate's impl, `sort` of its own `Ord`,
  gives that impl its `Borrow`'s dictionary, and works too.
- **Still errors:** a type's own `BorrowMut`.

## Why

- **It's exact:** the `generic_borrow` corpus case compares `Vec`s of
  `String`s, numbers, `&str`s and the crate's types looked up through
  generic code, an `equivalent`-style blanket impl, `Borrow<[i32]>` of an
  array, a `Vec` and a slice, `borrow()` of std's and the crate's
  directly, and a `sort` through the crate's `Ord` of a `K: Borrow<str>`,
  with native Rust; diagnostics tests the lookups refused.
- **It's the JS a person writes:** the conversion given to the generic
  function, the value itself where there's nothing to convert.

## Alternatives

- **Look a map's key up by what it borrows as,** a second index of each
  key's `borrow()`, kept in step as the map changes. Not now: no crate
  measured needs it, and it's a design of its own.
