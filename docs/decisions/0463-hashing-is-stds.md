# 0463. Hashing is std's: its hashers, and what `Hash` writes to one

Status: Accepted. Supersedes [0168](0168-user-hash.md)'s "never lowered";
amends [0425](0425-a-crates-own-future-and-reader-are-allowed-never-lowered.md).
Extends [0121](0121-value-keys.md), [0141](0141-std-trait-objects.md).
Extended by
[0864](0864-a-type-equal-only-to-itself-keys-a-set-by-itself.md):
`ptr::hash` writes its pointer's address. Amended 2026-11-03: a
`PhantomData` is known by its lang item, and writes nothing (#514). Amended
by [1120](1120-a-hash-key-found-by-its-own-hash-and-eq.md): a hash map of
keys whose `Hash` or `Eq` is their own calls `hash` as std's table does.

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
  std type rust-js doesn't write, a `Duration`, a range. (A `Cow` and std's
  wrappers and enums are written since #1289, amended below.)

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

## Amendment: `BuildHasherDefault` (2026-11-10, #1155)

uuid's `test_compare` hashes its ids by
`BuildHasherDefault::<DefaultHasher>::default().hash_one(a)`, refused at
its `Default`. `BuildHasherDefault<H>` holds nothing but its hasher's type,
a `PhantomData`, so its `default()` is `undefined`, as an empty struct's
is (`StdItem::BuildHasherDefault`, `core`'s by its name). Its
`build_hasher()` is an `H::default()`. Its `hash_one(x)` is std's provided
body: an `H::default()`, `x`'s `Hash` written into it as `x.hash(&mut h)`
writes it, and its `finish()`. Of `DefaultHasher`, `new $SipHasher(1, 3)`.
Case N: the hashes are native Rust's.

Held by the corpus case `build_hasher_default.rs`, against native Rust in
both builds, and the mutations `build-hasher-default-refused`,
`hash-one-of-nothing` and `build-hasher-unrecognized`, caught.

## Amendment: `RandomState` (2026-11-12, #1420)

quickcheck's `Arbitrary` of a `HashSet<K, H: BuildHasher + Default>`, and
so itertools' `correct_permutations`, was refused at `RandomState`.
**A `RandomState` is `undefined`**, as a `BuildHasherDefault` is: it holds
only its two random keys, which no program can read but through the
hashes it builds. `new()`, `Default` and `Clone`, a dictionary's in
generic code too, give `undefined`; `{:?}` is std's `RandomState { .. }`,
`{:#?}` and a width the same, as `finish_non_exhaustive` of no field writes
it. Its `build_hasher()` is a `DefaultHasher`, `new $SipHasher(1, 3)`, and
`hash_one(x)` std's provided body over one (`built_hasher`, its
`BuildHasher::Hasher`). Rust's keys are random, a new pair each run and
each `RandomState`, so a hash a `RandomState` builds is no value a program
can depend on, and rust-js's fixed keys give none it could tell from
Rust's: ADR 0262's "none" row, JavaScript's behavior. Two hashers of one
`RandomState` agree, as Rust's do. A `HashMap`'s or a `HashSet`'s
`with_hasher(s)` and `with_capacity_and_hasher(n, s)` of a `RandomState`
are its `new()` and `with_capacity(n)`, `s` made for what it does alone,
and its `hasher()` is `undefined` (`MapOwner::random_state`, of the type
after its keys and values, before its allocator). A table of another
hasher, and a `BuildHasher` method generic code calls of its type
parameter, stay refused.

Held by the corpus case `random_state_holds_nothing.rs`, against native
Rust in both builds and through the metamorphic net, and nine mutations,
caught.

## Amendment: std's ranges (2026-11-11, #1361)

itertools' probe, once #1346 let it compile, was refused at "implementation
evidence for `<Range<i32> as Hash>`", and a `HashSet<Range<i32>>` too: no
range had a `Hash` rust-js writes. std's ranges derive theirs, so a range
writes each bound in its field's order, `start` then `end`, `a..` its start,
`..b` and `..=b` their end, `..` nothing; and `a..=b` then its `exhausted`,
a `bool`. Each is what `range_state` says Rust's range holds, the one
answer `==` reads too: of one stepped to its end, its bounds and `true`,
so it hashes, as it compares, otherwise than a fresh one of its bounds.
`Hashed::Range(kind, bound)` (`recognition/types.rs`'s `hashed_as`) says
it, and a range's bounds whose own `Hash` runs code make it a key found
by its own `Hash` and `Eq` (ADR 1120). Case N: the hashes are native
Rust's.

Held by the corpus case `ranges_hashed_as_std_derives.rs` (each kind's
hash, keys of a set, a map's entry, generic code's, inside a derived key,
of `char`, `String` and printing bounds, an `a..=b` stepped to its end and
one past its type's last) against native Rust in both builds, and the
mutations `range-hash-unrecognized`, `range-hash-bounds-unwritten`,
and `range-hash-exhausted-unwritten`. Not mutated: whether a range's
`hash` runs code, its bounds', which nothing reads that sees it: a range is
never a key JS finds by value, so a map of them finds them by their own
`Hash` and `Eq` whatever it says, as std's generic branch said of
`Range`'s `Idx: Hash` before.
The case hashes no slice: its length is a `usize`, four bytes as
`wasm32`'s, where the host's native run writes eight.


## Amendment (2026-11-12, #1289): std's wrappers and enums hash as std's impls write them

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept. A hash table keyed by one of std's types around a
key whose `Hash` is its own, `HashSet<Reverse<Version>>`, was refused (ADR
1120): rust-js wrote no `Hash` of the std type, and a `$KeySet` that never
called the key's `hash` would print or panic differently. Each is now
hashed as std's impl writes it, so the key's `hash` runs as often as std
runs it (`hashed_as`, recognition/types.rs; `hash_writes`, std_types/hash.rs):

- one of std's that's its fields in JS, `Reverse(x)` and `Wrapping(x)`,
  `[x]`, whose `Hash` is derived: each field, in order, as a tuple's;
- one that's in JS the value it holds, a `ManuallyDrop` and the
  `MaybeDangling` in it: that value, as std's `self.value.as_ref().hash(state)`;
- a `Pin` of std's pointer, its pointer in JS (ADR 0329): its target, as
  std's `Ptr::Target::hash` and the pointer's own both write;
- a `Cow`: what it derefs to (`cow_deref`), as std's `Hash::hash(&**self)`;
- one of std's enums whose `Hash` is derived, `Bound`, `Poll`,
  `ControlFlow`, `io::ErrorKind`: its discriminant as its repr's integer,
  an `isize` unless it says otherwise, where it has more than one variant,
  then its variant's fields in order, as `#[derive(Hash)]` writes. Each
  variant is told by `variant_test` and each field read by
  `field_of_variant` (representation.rs), the enum arm `mir_field` had, so
  a pattern and a hash read a variant by one owner.

A tuple's parts, a `Wrapping`'s, of an array made where it's hashed that
does nothing are written as they are, `H.writeU32(state, counted[0])`,
never the array held and read back; one made of calls is held first, its
calls made before any part's `hash` runs, as std makes the tuple first.
`hash_of_a_wrapping_in_generic_code_refused.rs` compiles, renamed without
`_refused`; and a `Cow` of a slice of what's equal only to itself, refused
under ADR 1015, is a `$hashedSet`'s key, each found by its own `Hash`.

So `Bound::Excluded(v)` hashes as `Some(v)` does, natively and in JS. What
the hash comes to differs from a 64-bit target's wherever the discriminant,
an `isize`, is written, as this ADR's Consequences say of every `usize`.

Still refused, each naming its issue: a `Saturating` (#1434), which rust-js
didn't represent, its arithmetic refused as "calling `std::ops::Add::add`
(#1434)"; and a `Pin` of the crate's own pointer (#1435), whose `Hash` is
its target's, not the pointer's own. (Since #1434 a `Saturating` is `[x]`,
hashed as its derived `Hash` writes, as a `Wrapping` is: ADR 0175, amended;
since #1435 a `Pin` of the crate's own pointer hashes its `deref()`'s
target, `Hashed::Target`: ADR 0329, amended. What's left names #1447.)

Held by corpus case `std_keys_around_their_own_hash.rs` against native Rust
in both builds: each wrapper and enum as a key, its key's `hash` calls
counted, and hash equalities, not values, which are a target's. Diagnostics
rows of the three refusals. Mutations `std-struct-hash-unwritten`,
`std-enum-hash-unwritten`, `std-wrapped-hash-unwritten`,
`pin-hash-unwritten`, `pin-of-any-pointer-hashed-as-it`,
`cow-hash-unwritten`, `std-enum-discriminant-unwritten`,
`std-enum-discriminant-zero`, `std-enum-hash-runs-no-code`,
`hashed-made-array-parts-with-effects`, `hashed-made-array-held`.

## Amendment (2026-11-12, #1447): every std type hashed as std's impl writes it

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is kept; for `TypeId`, none. Hashing a std type rust-js
didn't write was refused, alone and in a key beside one whose `Hash` runs
code. Each is now written as std's impl does (`hashed_as`, `hash_writes`):

- A `Duration`, an `Instant` and a `SystemTime`, each its nanoseconds, a
  BigInt (ADRs 0188, 0447): its seconds, a `u64`, then the nanoseconds under
  one, a `u32`, as `Duration`'s derived `Hash` and wasm32's `Instant` and
  `SystemTime`, which hold one or a timespec, write them; a `SystemTime`'s
  seconds floored, an `i64`, its nanoseconds never negative, as wasm32's
  timespec has one before the epoch.
- An `OsStr` and an `OsString`, its text: its UTF-8 bytes as a `[u8]`, as
  `as_encoded_bytes().hash()` does.
- A `Path` and a `PathBuf`: std's component-wise `Hash`, `$hashPath(path,
  state, H)` (runtime hasher.js): each part's bytes between separators, a
  `.` after one left out, then the rotated sum of their lengths, a wasm32
  `usize`.
- A `Discriminant<T>`, its number, as `T`'s discriminant type; a std struct
  with no fields whose `Hash` is derived, `PhantomPinned`, nothing; a
  `Layout`, its size then its alignment, each a `usize`.
- A `TypeId`, its type's name (ADR 0331), by `write_str`. std hashes an id
  the compiler picks, documented as unspecified and changing between
  compilers, so no program can rely on native's raw value; what std does
  say, equal ids hash equal, holds. ADR 0262's "none" row, as `RandomState`'s
  keys are (m6, on the user's behalf); listed in the guide's differences.

Held by corpus case `std_types_hashed_as_std_does.rs` against native wasm32,
raw `DefaultHasher` values compared, a `TypeId`'s equalities, and each in a
tuple key beside one that counts its `hash` calls, both builds. Mutations
`clock-hash-unwritten`, `system-time-hash-truncated`,
`os-text-hash-unwritten`, `discriminant-hashed-as-u8`,
`fieldless-std-hash-unwritten`, `path-hash-unwritten`,
`layout-hash-unwritten`, `type-id-hash-unwritten`,
`layout-hash-align-first`, `path-hash-dot-kept`, `path-hash-unrotated`.
Nothing found is left refused, so the `(#1447)` tags, `open_evidence` and
the guide's limitation line go. And mutation `hash-runs-code-of-any-bound` goes: it held
`hash_runs_code`'s fallback for std's impls, which counts only `Hash`
bounds, by a `Discriminant<T>`, whose impl's `T: Sized` names what it never
hashes; a `Discriminant` is hashed directly now, and with that filter gone
the whole corpus and every crate pair still pass, no other std type
reaching it but `NonNull`, which rust-js refuses. The filter stays, as
std's rule.
