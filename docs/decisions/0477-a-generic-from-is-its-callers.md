# 0477. A generic `From::from` is its caller's

Status: Accepted. Extends [0108](0108-generic-operators-and-into.md). Amended: an impl's method nothing calls is written nowhere, below.

Case: A ([0262](0262-when-rust-and-js-disagree.md)): the same value, made
by the same conversion.

## Context

`x.into()` of a `T: Into<U>` is its dictionary's `into`, std's conversion
or the crate's `From` impl (ADR 0108). `W::from(u)` of a `W: From<U>` was
refused, `From` being a trait whose impls rust-js lowers but whose bounds it
gave no dictionary. itertools calls one of an associated type a trait bounds
by it, `T::Combination::from(iter)` of `type Combination: From<I>`, and of
what a bound names, `Zip::from(t)` of `Zip<T>: From<U>`.

## Decision

**`W::from(u)` of a `W: From<U>` is its dictionary's `from`**, its
caller's: `From` is `operational`, and its dictionary `{ from }` is made as
`Into`'s `{ into }` is, std's conversion or the crate's impl's function. An
associated type's bound by it is its trait's dictionary's, as any of its
bounds is. Where the types are known, or no bound gives it, `VecDeque::from(v)`
of a `Vec<T>`, std's impl wherever it's called, it's written in place, as
ever, and a crate's impl's dictionary is written only where generic code
reads it (`read_only_generically`).

```js
function build(u, WFromU) {
  return WFromU.from(u);
}

build([3], { from: wrapFromVec_from });
conv(7, { from: (n) => n });
```

## Why

- **It's the same conversion**: `generic_from` converts by the crate's
  impl, by an associated type's bound, and by std's, `u32: From<u8>` and
  `String: From<&str>`, through generic code, with native Rust.

## Consequences

- **What compiles grows**: `From::from` through a bound.

## Amendment: an impl's method nothing calls is written nowhere

Case ([0262](0262-when-rust-and-js-disagree.md)): none; a function nothing
calls is no function of the JS.

React's reconciler reads a lane back as an `EventPriority`, its
`ReactEventPriorities`' newtype of one, by `impl From<Lane> for
EventPriority`, which React DOM calls and nothing of the reconciler does.
Its module printed `function eventPriorityFromI32_from(lane) { return lane;
}`, which upstream's hasn't and nothing reads: the impl's dictionary was
written only where generic code reads it, but its method everywhere.

**An impl of a trait only generic code reads a dictionary of
(`read_only_generically`: `From`, the operators, `Deref` and the others) is
written only where it's reached, its methods with it**: each is called as
it is, by its name, where anything calls it, which reaches it. A consumer
calling a library's compiles its own, as of any impl's method its library's
JS doesn't export (ADR 0914, amended): `const priority =
eventPriorityFromI32_from(8);` and the function beside it. So corpus
`untagged_enums`' `valueFromString_from` and `deref_projection_generic`'s
`stackDeref_deref`, which nothing called, are gone.

### What holds it

- `test/crates/pairs/from_impl_unused`, run against native Rust;
  `test/crates.test.ts`, "a library's From impl nothing of its own calls is
  a consumer's to compile".
- Mutation `generic-only-methods-written-unreached`
  (`lower/pipeline/reached.rs`).
