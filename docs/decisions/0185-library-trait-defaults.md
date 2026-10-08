# 0185. A library's trait's defaults are its functions over `Self`

Status: Accepted. Extends [0100](0100-separate-crates.md),
[0049](0049-traits-and-generics.md), [0174](0174-generic-fmt-traits.md)
and [0019](0019-one-js-file-per-module.md).

Case: N, C ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A default method's body is copied into each impl of its trait, with `Self`
the impl's type (ADR 0049). A consumer can't copy a library's: rust-js
lowers rustc's THIR, which a dependency's metadata doesn't have. So an
impl of another crate's trait had to write every default itself (ADR 0100),
and bitflags' macro, which implements bitflags' `Flags` for the crate's
type, writes three of its 30 methods.

## Decision

**A library's trait's default is a function of the library's, generic over
`Self` as any generic function is: given `Self`'s dictionary, and its
drop. A consumer's impl that doesn't write it calls it, given its own
dictionary:**

```js
// dep
export function counter_double(self, SelfCounter, dropSelf) {
  return Math.imul(SelfCounter.get(self), 2) >>> 0;
}
```

```js
// app
export function oneCounter() {
  if ($oneCounter === undefined) {
    $oneCounter = {
      get: oneCounter_get,
      double: (arg0) => counter_double(arg0, oneCounter()),
    };
  }
  return $oneCounter;
}
```

- **A call on `Self` in it is its dictionary's,** so an impl's override is
  what a default calls, as in a copy.
- **It's in the library's manifest as its other functions are,** with the
  drops it takes; a consumer calls it as it calls a library's generic
  function, with the dictionaries its bounds ask for.
- **Within a crate, a default is still copied:** a call on `Self` is then a
  direct one.

What bitflags' macro needs besides:

- **std's `LowerHex`, `UpperHex`, `Octal` and `Binary` of an integer, given
  a `Formatter`,** `fmt::LowerHex::fmt(&self.0, f)`: its digits, with the
  prefix where the `Formatter` is alternate, padded by its options as they
  are at run time, `$formatted((options?.alternate ? "0x" : "") +
  bits[0].toString(16), options, true)`. Where a `T: LowerHex` goes, a
  dictionary of that.
- **A module in a block is a file of its own:** bitflags' macro writes a
  `mod __bitflags_flag_names` in a function, and another in a constant.
  One of the same name as another of its parent's is numbered in the order
  they're written, `__bitflags_flag_names$1.js`, after the one the parent
  declares, which keeps its name. No Rust module's name has a `$`.
- **The drop of an `impl Trait` argument is named for its trait,**
  `dropDisplay`: rustc names its type parameter as it's written, `impl
  fmt::Display`, which isn't JS.

## Why

- **It's exact:** a two-crate test compares a library's defaults called
  through a consumer's impl, directly, in the consumer's generic code and
  the library's, an override a default calls, a default that drops `self`
  and one of `&mut self`, with native Rust.
- **A crate using bitflags runs as natively:** a `bitflags!` type's
  operators, `Debug`, `from_name`, iterators, `to_writer` and parser, each
  line the same, compiled with bitflags, every crate by rust-js.
- **It's what a generic function already is:** the body is lowered as one,
  and the dictionary entry is what a call of one is.

## Alternatives

- **The library's THIR in its manifest,** for its consumers to copy:
  rust-js's own serialization of rustc's types, for every crate, and a
  default's body lowered again in each impl.
- **A default's JS in the manifest, specialized by the consumer:** text
  rewritten across a crate boundary, which ADR 0100 keeps out.

## Costs

- **A library's default is called through its dictionary,** where a copy
  would call the impl's method directly.
- **A library ships each reachable default,** used or not.
- **A std dictionary of an integer's `LowerHex` is written where it's
  given,** as one of its `Debug` is: `{ fmt: (value, options) => .. }`.
