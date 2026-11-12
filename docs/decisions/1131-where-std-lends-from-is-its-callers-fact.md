# 1131. Where std lends from in generic code is a fact its caller gives

Status: Accepted (2026-11-12, #1478). The coordinator approved the design:
one fact, `TypeFact::LentOffset(Lend)`, asked only where instances disagree.
Beside [0562](0562-a-pointer-is-a-place-and-an-address.md), pointers, and
[0145](0145-type-facts.md), type facts; classified per
[1129](1129-a-fact-of-a-librarys-contract-says-what-makes-it-true.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change (a
pointer's address, and so its arithmetic and comparisons), so Rust's
layout is preserved.

## Context

In a crate that asks an address, a `&mut` std lends from inside its
receiver, `Result::as_mut`'s `Ok` payload, is a handle on its place there,
`$ptr(r, "_0", offset)` (#1474), and the offset is rustc's layout of the
receiver's type. Generic code `fn ok_ptr<T, E>(r: &mut Result<T, E>)`
knows no single offset: a `Result<u8, u16>` keeps its `Ok` at 1 and a
`Result<u16, u8>` at 2. Where every instance agreed, the one offset was
used; where they disagreed, the lend was refused (#1478). A library's
generic lend was refused too, as it sees none of its consumers.

## Decision

The offset is a type fact of the receiver, `TypeFact::LentOffset(Lend)`,
its caller's to give, as `Size` is. `Lend` names what's lent: `Ok`,
`Err`, `OnceCell`, `LazyCell` and `Peekable` (#1513 extends the enum,
not the fact list). The receiver is a type built of the caller's type
parameters, `Result<T, E>`, so it's a built slot (#737), which each caller
fills with rustc's layout of its instance: `okPtr(a, 1, 2)` and
`okPtr(b, 2, 1)` give `ResultOkOffset` and `ResultErrOffset`.

- What std lends, and where, has one owner, `recognition/lends.rs`
  (`lends_mut_of`, `lend_offset`), which the crate's analysis and the
  lowering both ask.
- The analysis asks the fact only where the instances it sees keep the
  lend at different offsets, or where it sees none, a library's: no other
  output changes, and no existing snapshot moved.
- A caller that can't give it, of a type with no layout, is refused at
  its call (`type_fact_value`), as one that can't give `Size` is.
- In the library contract, it rides `TypeParam.facts` and
  `TypeParam.built`, true of the consumer's instance.

## Consequences

- Generic code's lend from a `Result` compiles where its instances
  disagree, and a library's from its consumers' instances.
- A generic `OnceCell<T>`, `LazyCell` or `Peekable` lend whose value JS
  keeps in a `$some` box stays refused (#1513).
- Proof: corpus case generic_lend_offsets_given_by_callers and crate pair
  lend_offset_a_consumer_gives against native wasm32; mutations
  lend-offset-never-asked, lend-offset-unseen-callers-unasked,
  lend-offset-given-unread and lend-offset-given-zero, caught.
