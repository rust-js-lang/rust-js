# 0400. A `Cow` compares and orders as what it borrows

Status: Accepted. Extends [0319](0319-cows.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): std's `PartialEq`,
`Ord` and `PartialOrd` of a `Cow` deref it, so a borrowed one equals an
owned one of the same text; a comparison of its enum would tell them
apart.

## Context

A `Cow` is its enum, `{ TAG, _0 }` (ADR 0319). `==` and ordering of one
were refused, as of any std enum's, and a struct that derives its `==`
around one, or an `Option` of one, had no way to compare it. Rust compares
what it borrows, beside a `&str` or a `String` too: `Cow::Borrowed("a") ==
Cow::Owned("a".into())` and `cow == "a"`.

## Decision

**A `Cow` is compared, ordered and sorted as what it borrows, its `_0`,
`cow_deref`'s one answer, of its borrowed type: beside another `Cow`, or
beside what it borrows, `cow == "a"`, `"a" == cow`.**

```js
console.log(`${borrowed._0 === owned._0}`);
words.sort((a, b) => $cmp(a._0, b._0));
`${a.rank === b.rank && a.name._0 === b.name._0}`;
```

- **What holds one compares it so**: a `Cow` compares by its own rule, so
  an `Option` of one, or a struct deriving its `==`, compares it part by
  part, not as `$eq` of its object, whose `TAG` differs.
- **Still refused**: a `Cow` as a map's key, whose map would find a
  borrowed one by what it borrows, and give back the key it holds, not
  the one asked with.

## Why

- **It's the program Rust runs**: the corpus's `cow_compare.rs` compares
  borrowed and owned text, beside `&str` and `String`, behind
  references, sorts a `Vec` of both, takes its `max`, and derives `==` and
  `Ord` around one, and compares `Cow<[i32]>`s, each with native Rust.
