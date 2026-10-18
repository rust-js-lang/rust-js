# 0445. Generic code extends and collects into what it's given

Status: Accepted. Extends [0062](0062-combinators.md),
[0098](0098-destructors.md), [0106](0106-generic-traits.md),
[0160](0160-user-collections.md) and [0180](0180-generic-writers.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same program,
each value where Rust puts it.

## Context

Shared models are written against traits: either's `Either` extends and
collects into whatever its sides are, `impl<L: Extend<A>, R: Extend<A>>
Extend<A> for Either<L, R>`, and itertools builds on either. Both stopped
at rust-js's first refusal, `calling Extend::extend`, and behind it were
more, each of a common idiom:

- `s.extend(chars)` of a `String`, refused even where the type is known.
- `c.extend(items)` of a `C: Extend<A>`, and `collect()` into a `B:
  FromIterator<A>`, `B::from_iter(items)`: generic code had no dictionary of
  either trait.
- `nth`, `find` and `partition` of items with a destructor, refused because
  std drops what they pass over, which rust-js didn't.

## Decision

**`Extend` and `FromIterator` are given dictionaries, as `AddAssign` is
([0396](0396-a-compound-assignment-in-generic-code.md)): generic code calls `extend` and `from_iter` through them, and
a known collection's call is written in place, as before.**

```rust
fn fill<C: Extend<u32>>(c: &mut C, n: u32) { c.extend(0..n); }
fn gather<B: FromIterator<u32>>(n: u32) -> B { (1..=n).map(|x| x * 2).collect() }
```

```js
function fill(c, n, CExtendU32) {
  CExtendU32.extend(c, $lazyRange(0, n));
}
function gather(n, BFromIteratorU32) {
  return BFromIteratorU32.from_iter($lazyRange(1, n + 1).map((x) => Math.imul(x, 2) >>> 0));
}
gather(3, { from_iter: (items) => Array.from(items) });
```

- **A std collection's dictionary is what its known call is**, of the
  items given, any iterable: a `Vec`'s `$extend`, a map's or a set's
  `$extendMap`, a `String`'s text added; `from_iter` what `collect` into it
  makes. Its `extend` takes the box generic code's `&mut` is (ADR 0180).
  One function answers what a std collection's `extend` is,
  `Recognition::std_extend`, which the known call and the dictionary ask.
- **The crate's own impl is its dictionary**, written only where generic
  code reads it (`read_only_generically`), as an operator's is.
- **A range or an iterator of the crate's given a std method's own generic
  `IntoIterator`**, `c.extend(0..n)` through a dictionary, is a JS iterator,
  as one given a function of the crate's is (ADR 0061).
- **`s.extend(items)` of a `String` is a `push_str`** of their text,
  `s += ["y", "z"].join("")`; a range's or an iterator's, `Array.from(..)`
  of it, joined.
- **What a method passes over of items with a destructor it drops**, as
  std's does (ADR 0098): `nth(n)` each it skips, `$nthDropping`, and
  `find(test)` each it rejects, `$findDropping`; `partition` moves each into
  one of its two, as `extend` does, and drops none.

## Why

- **It's Rust's**: generic code over `Extend` and `FromIterator` is how a
  shared model, and the crates it uses, are written; `s.extend(chars)` is
  how Rust builds text.
- **One form for each**: a known collection's call is written in place, a
  generic one's through its dictionary, as every operational trait is.
- **It's tested**: corpus cases extend and collect generically into std's
  collections, text, a `Result` and the crate's own, and drop what `nth`
  and `find` pass over, each against native Rust; a mutation for each step
  is caught.

## Consequences

- either compiles past them, and stops at `rfind`, `nth_back` and `rfold`,
  a double-ended iterator's, which rust-js doesn't lower yet.
- Giving a generic function an iterator of items with a destructor where
  it takes no drop for it is still refused (ADR 0190).
