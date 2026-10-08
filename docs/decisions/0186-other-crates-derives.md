# 0186. What another crate's derive writes is the crate's own code

Status: Accepted. Amends [0052](0052-std-trait-impls.md) and
[0060](0060-debug.md); extends [0141](0141-std-trait-objects.md) and
[0034](0034-strings-and-chars.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

rust-js has std's derives' meaning itself: a derived `Clone` is written in
place (ADR 0052), a derived `PartialEq` is JS's comparison, and the impl
rustc expands is never lowered. It knew one by `#[automatically_derived]`.

But a derive of another crate's writes that too, as strum's and
thiserror's do, and what they write is code no one else has: strum's
`IntoEnumIterator` for an enum, thiserror's `From` for `#[from]`. Skipped,
a crate that used them was refused, though both crates compiled: the crate
corpus's probes, run as JS beside native Rust, found it.

## Decision

**A derive whose meaning rust-js has itself is std's own, a built-in macro.
Any other's impl is the crate's own code: lowered, given as a dictionary,
and validated as one written by hand.** rustc says which:
`is_builtin_derived`, an impl a `#[rustc_builtin_macro]` derive expanded.

- **serde's is left out where it's read,** as before: its codecs are
  rust-js's (ADR 0077).
- **Another crate's impl is as it says,** `#[automatically_derived]`:
  rustc knows which macro wrote only the crate it compiles, and only that
  crate's impls are lowered here. core's derived `Debug` of `IntErrorKind`
  is still std's.

What strum's and thiserror's derives need besides:

- **`Display` of a `fmt::Arguments` is its text,** which `format_args!` is
  (ADR 0034), whatever width the `Formatter` has, as std's writes it:
  strum's `Display::fmt(&format_args!("circle of {}", radius), f)`.
- **std's `Error` of one of its parse errors is a dictionary,** its
  `Display` and `Debug`, and no `source`, which none of them has: where a
  `ParseIntError` is given for an `E: Error`, as thiserror's
  `#[error(transparent)]` gives it to its blanket `AsDynError`.

## Why

- **It's exact:** a Cargo workspace with a derive of its own, an impl of a
  library's trait called directly and in generic code, and a `From` that
  `?` uses, prints what native Rust does; the corpus's strum and thiserror
  probes run as natively.
- **It's what rustc says,** not a list of crates: a derive no one has
  heard of is code like any other.

## Alternatives

- **A list of the derives rust-js has:** std's by name, then serde's. A new
  std derive, or another crate's of the same name, would be read wrong.

## Costs

- **Another crate's derive of a std trait, `Clone` say, is its code,** not
  std's meaning: as Rust has it, but the JS is its body's, not a copy in
  place.
- **std's `Error` dictionary is written where it's given,** as its `Debug`
  and `Display` are.
