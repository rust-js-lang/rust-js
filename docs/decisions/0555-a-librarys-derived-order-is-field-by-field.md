# 0555. A library's derived order is field by field

Status: Accepted. Extends [0057](0057-ordering.md) and
[0100](0100-separate-crates.md). Amended 2026-11-03: a `PhantomData` is
`Equal`, a part of no order (#514).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a derived `Ord` is
its fields' in turn, wherever its type is declared.

## Context

uuid's `Uuid` derives `Ord`; `ids.sort()` of `Uuid`s was refused,
"comparing `uuid::Uuid`s". The crate's own struct with a derived order is
compared field by field in place; another crate's was refused, as its
impl may not be field by field.

## Decision

**A struct of another crate whose `Ord`, or `PartialOrd` where that's
what's asked, rustc says is a `#[derive]`, `derives`, is compared field by
field, as the crate's own is.** One written by hand is still refused.

## Why

The two-crate test `derived_order` sorts and takes the `max` of a
library's `Id([u8; 2])`, and asks a library's `Version` with an `f64`,
which derives only `PartialOrd`, its order, unordered too, against native
Rust; uuid's probe sorts `Uuid`s and runs as native.

## Consequences

- uuid runs.
- Mutations refuse a library's derive, and ask a `PartialOrd`'s as an
  `Ord`'s.

## Amendment: a `PhantomData` is `Equal`, a part of no order (#514)

Case: N ([0262](0262-when-rust-and-js-disagree.md)): a refusal lifted;
the order is std's.

`map.cmp(&other)` of a struct deriving `Ord` that holds a `PhantomData`,
litemap's `LiteMap`, was refused, "comparing `PhantomData<str>`s": std's
`PhantomData` is another crate's struct whose `Ord` isn't derived, which
this decision refuses, as its impl may order otherwise. Its impl is
std's, and every one is `Equal`: it orders as a type of one value, `()`
or `struct Zst;`, does, `0`.

**A `PhantomData` is `Equal`, and a part of one value is no part of its
struct's order:** `$cmpItems(a.values, b.values, ..)`, not
`$cmpItems(..) || 0 || 0`.

- `test/corpus/hash_phantom_data.rs`: `cmp` and `<` of litemap's struct,
  its `BTreeSet`, and `cmp` and `<=` of two `PhantomData<str>`s, as
  natively.
- Mutations `phantom-data-order-refused` and `one-value-part-ordered`
  (`ordering.rs`).
