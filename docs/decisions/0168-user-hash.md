# 0168. A type's own `Hash` is allowed, and never lowered

Status: Accepted. Extends [0121](0121-value-keys.md).

Case: A, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A type hashes itself where its derive can't, or shouldn't: semver's
`Identifier` hashes the text it packs, arrayvec's `ArrayVec` its items,
uuid's `Uuid` its bytes.

```rust
impl Hash for Identifier {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.as_str().hash(hasher);
    }
}
```

That was an error, and stopped semver, uuid, arrayvec and powerfmt, so
rust_decimal and time.

## Decision

**A type's own `impl Hash` is allowed, and, like a derived one, never
lowered:** its methods are no JS.

- **A map keys by value,** a `$KeyMap` for a struct, tuple or enum key with
  a derived `Eq` (ADR 0121): it compares keys and never calls `hash`. Rust
  requires `a == b` to mean `hash(a) == hash(b)`, so which keys a map
  holds is `Eq`'s alone, and the impl changes nothing a program sees.
- **A key whose `PartialEq` is the crate's is still an error,** as before:
  that's the `Eq` a map would need to call.
- **Calling `hash` is still an error,** as it was of a derived one: there's
  no `Hasher` to call it with, std's `DefaultHasher` or the crate's own.

## Why

- **It's exact:** the `user_hash` corpus case compares maps and sets of
  structs, enums and a generic type's two impls that hash less than `==`
  compares, with native Rust.
- **It's no JS at all,** as a derived `Hash` is.

## Alternatives

- **Lower the impl and call it,** a map hashing as Rust's does. Not now:
  it needs a `Hasher` in JS, and a map's answers would be the same.
