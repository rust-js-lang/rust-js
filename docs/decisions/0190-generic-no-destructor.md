# 0190. Generic code may take no destructor of a type parameter, and its callers are checked

Status: Accepted. Extends [0098](0098-destructors.md) and [0100](0100-separate-crates.md).

## Context

rust-js runs a value's destructor where Rust does (ADR 0098). A generic
function's type parameter may be given one with a destructor, so its drop
is the caller's to give, `dropT`, and what rust-js can't drop of one is an
error: a `for` loop over a generic iterator, which drops what it hasn't
reached when it leaves early, a `fold` of one, `Vec::clear` of a `Vec<T>`.

chrono's are such, its parser's and formatter's loops and its `Sum`s'
`fold`s, five places, though nothing it's given has a destructor: a
library's type parameters may be given one by a consumer.

## Decision

**What a generic function would drop of a type parameter, and can't, is
taken as nothing, and the function then takes no destructor of that
parameter. A call that gives it one is an error, at the call:**

```text
error: rust-js does not support giving `total`'s `I` a type with a destructor, where it takes none yet
 --> src/lib.rs:5:21
  |
5 | pub fn f() -> u32 { total(Counter(0)) }
```

- **Only where it would be an error:** a loop over a generic iterator, a
  std call that would drop what it's given, and only where what it would
  drop is the type parameters' alone, `I` or a `Vec<T>`, not a type's of
  its own.
- **A caller that gives its own type parameter takes none of it too,**
  `relay<J>` calling `total(j)`: the crate's calls are checked once each
  function is lowered, as one may be lowered before what it calls.
- **A trait impl's method takes none of the impl's parameter where its
  dictionary is made too:** the dictionary gives each method the impl's
  parameters, a call like any other, so a call through one, `e.empty()` of
  an `E: Empty`, is checked where the caller makes `Wrap<Loud>`'s.
- **A library's are in its manifest,** `no_drops`, and its consumers'
  calls are checked as they're lowered.

## Why

- **It's exact:** nothing is dropped wrong, as nothing given has one; and
  what would be is an error, at the call that gives it, which is where a
  reader looks. The `generic_iterator_items` corpus case loops over, steps
  and folds generic iterators, as chrono does, with native Rust, and the
  diagnostics and crate tests check each call that gives one is refused:
  directly, through another generic function, through a dictionary, and
  of a library's.
- **chrono's five places compile,** and the generic code of any library
  that's given no destructors.

## Alternatives

- **Drop what's left of a generic iterator:** the iterator's drop, a
  caller's, of the JS iterator it's been made, and that of what it holds,
  for std's `vec::IntoIter` of them too, which rust-js can't follow (ADR
  0098). The whole design, for callers chrono doesn't have.

## Costs

- **A call Rust takes may be refused,** where it gives one with a
  destructor: a `for` loop over an iterator of them in generic code.
- **A requirement is the parameter's, not the place's:** one loop over `I`
  takes none of `I` for the whole function.
