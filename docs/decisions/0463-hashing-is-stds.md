# 0463. Hashing is std's: its hashers, and what `Hash` writes to one

Status: Accepted. Supersedes [0168](0168-user-hash.md)'s "never lowered";
amends [0425](0425-a-crates-own-future-and-reader-are-allowed-never-lowered.md).
Extends [0121](0121-value-keys.md), [0141](0141-std-trait-objects.md).
Extended by
[0864](0864-a-type-equal-only-to-itself-keys-a-set-by-itself.md):
`ptr::hash` writes its pointer's address. Amended 2026-11-03: a
`PhantomData` is known by its lang item, and writes nothing (#514).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same hash, bit for
bit, as `wasm32`'s native Rust, rust-js's target (ADR 0090).

## Context

A program hashes where it wants a number for a value: a cache key, a
checksum, a fingerprint sent to a server whose Rust hashes the same value,
a hasher of its own, FNV or a counter of writes, in place of std's.

```rust
let mut hasher = DefaultHasher::new();
key.hash(&mut hasher);
let id = hasher.finish();
```

ADR 0168 allowed a type's own `Hash` and never lowered it: a map keys by
value (ADR 0121) and never calls `hash`, so there was no hasher to call it
with. `DefaultHasher`, `SipHasher` and a crate's own `Hasher` were refused,
and with them a shared model's fingerprint.

## Decision

**std's hashers are a `$SipHasher`, and `Hash`'s and `Hasher`'s methods are
lowered as any trait's: each write std's impls make, to any hasher, makes
the same `finish()` as native Rust's.**

```js
function pointHash_hash(point, state, HHasher) {
  HHasher.write_i16(state, point.x);
  HHasher.write_str(state, point.name);
}

const hasher = new $SipHasher(1, 3);
hasher.write_u32(5);
hasher.finish(); // 14828406784900857967n, as std's
```

- **`DefaultHasher` is SipHash-1-3 of zero keys, and `SipHasher` SipHash-2-4**,
  of its keys where `new_with_keys` gives them: `new $SipHasher(1, 3)`, over
  64-bit BigInts. `new()` and `default()` make one (`new_hasher`); `clone()`
  copies its state; `finish()` doesn't end it, as std's doesn't.
- **std's types write what std's impls write**: an integer its bytes,
  little-endian, a `usize` four, as `wasm32`'s; a `bool` a `u8`; a `char` its
  code point, a `u32`; text its bytes and `0xff`; a slice, an array, a `Vec`
  and a `VecDeque` their length first; an `Option` and a `Result` their
  discriminant, an `isize`, then what they hold; a reference, a `Box`, an
  `Rc` what it points at; a `NonZero` its number, an `Ordering` its `i8`. An
  integer slice is one `write` of its bytes, as std's `hash_slice` makes it,
  which a crate's hasher can tell from several.
- **A derived `Hash` is lowered**, as a derived `Debug` is (ADR 0060): its
  discriminant, `discriminant_value`, the integer its `repr` says, a 64-bit
  one a BigInt, then its fields'. A derive's own `__H` is named `H`.
- **A crate's `Hasher`'s dictionary is `$hasher({ finish, write, .. })`**:
  std's provided methods, `write_u32` to `write_str`, filled in, each std's
  over the dictionary's own entries, so `write_i64` writes through the
  impl's own `write_u64` as std's does. `Hash`'s provided `hash_slice` is
  `$hashSlice(hash)`, so a type's own `hash_slice` is the one a slice of it
  calls.
- **Generic code is given dictionaries as for any trait**: a std hasher's is
  `$sipHasher`, its own methods; a std type's `Hash`'s is `{ hash, hash_slice
  }`, the writes above; `&mut H`'s and `Box<H>`'s are what they point at's,
  a `dyn Hasher` too, a pair (ADR 0141). Where the types are known, a call
  is written in place, `hasher.write_u32(n)`, as a writer's is (ADR 0180).
- **A `Hash` and a `Hasher` impl are written only where they're reached**, as a
  derived one is: a map keys by value and never calls `hash` (ADR 0121), so
  ADR 0168's promise holds, an impl nothing hashes with is no JS.

## Why

- **It's std's answer, bit for bit**: `hash_values` and `hash_hashers`
  compare every `finish()` and every write a crate's hasher is given with
  native Rust built for `wasm32-wasip1`, whose `usize` is rust-js's.
- **One way for each job**: a hash call is a trait call; the provided
  methods are std's code once, `$Hasher`'s, which a std hasher inherits and a
  crate's dictionary is filled from.

## Consequences

- **A 64-bit target's hash isn't rust-js's** where a `usize` is hashed: a
  length, an index. It's `wasm32`'s, as every `usize` of rust-js's is (ADR
  0090), so a server compiled for a 64-bit target hashes a `Vec` differently.
- **Still refused**: `BuildHasher` and its `RandomState`,
  `BuildHasherDefault` and `hash_one`; `{:?}` of a std hasher; and hashing a
  std type rust-js doesn't write, a `Duration`, a range, a `Cow`.

## Alternatives

- **Keep ADR 0168, hashing refused.** A fingerprint is what a shared model
  sends a server; refusing it stops the port, not the program.
- **A hasher of JS's, FNV over strings.** Its numbers wouldn't be std's, so
  a value hashed in the browser and on the server wouldn't agree.

## Amendment: a `PhantomData` is known by its lang item, and writes nothing (#514)

Case: N ([0262](0262-when-rust-and-js-disagree.md)): a refusal lifted;
what's written is std's, nothing.

Since this decision, litemap, which url's graph builds, was refused at
`Hash::hash` of its map's `PhantomData<K>`: `#[derive(Hash)] pub struct
LiteMap<K: ?Sized, V: ?Sized, S> { values: S, _key_type: PhantomData<K>,
_value_type: PhantomData<V> }`. Before it, no `Hash` was lowered, and
litemap compiled. `hashed_as` had `PhantomData` writing nothing, as std's
`impl Hash for PhantomData<T>` does, but asked `StdItem::PhantomData`,
whose diagnostic item, `PhantomData`, std doesn't have: a `PhantomData` is
the lang item `phantom_data` alone. So the arm never matched, and the
marker's `hash` was taken for a crate's, whose dictionary isn't one.

**A `PhantomData` is known by its lang item, as every other module knew
it already, and its `hash` writes nothing:** `StdItem::PhantomData` is
gone, and with it serde's arm of it, which was never reached either, a
skipped field's default being `undefined` already. A slice of items that
write nothing is no loop, `hashSlice(items, state, H) {}`.

`test/architecture.test.ts`'s "each std item is known by what std gives
it" holds that each `StdItem` recognition asks of its diagnostic item is
one std's source gives, or that `is_std_def` knows it otherwise, by its
lang item or name, as `MaybeUninit`, `OnceCell` and `LazyCell`.

- `test/corpus/hash_phantom_data.rs`: litemap's struct, of unsized `K` and
  `V`, hashed through `Hash::hash`, as its store is, a slice of markers
  and a marker with a number, each `finish()` as native Rust's for
  `wasm32`.
- Mutations: `phantom-data-hash-unrecognized` (`recognition/types.rs`)
  and `nothing-written-looped` (`std_types/hash.rs`).
- The first bad commit, bisected: 88383e402, this decision's.
