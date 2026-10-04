# 0165. `{:x}`, `{:e}` and `{:p}` of the crate's types call its own impls

Status: Accepted. Extends [0054](0054-display.md) and
[0143](0143-formatter-options.md).

## Context

A type of the crate's shows by its own `Display` and `Debug` (ADR 0054).
Its other `fmt` traits, `LowerHex`, `UpperHex`, `Octal`, `Binary`,
`LowerExp`, `UpperExp` and `Pointer`, were errors, though ids and hashes
are shown in hex: uuid's `LowerHex` and thiserror's `Pointer` stopped them
(docs/crate-corpus.md). So was `fmt::Write::write_str(f, s)` called
through the trait on a `Formatter`, where `f.write_str(s)` wasn't.

## Decision

**A placeholder of one of them calls the type's impl, as `{}` calls its
`Display`, given the placeholder's options:**

| Rust | JS |
|---|---|
| `{:x}` of an `Id` | `idLowerHex_fmt(id)` |
| `{:#x}`, `{:>10b}` | `idLowerHex_fmt(id, { alternate: true, .. })`, its options given |
| `LowerHex::fmt(self, f)` in a `fmt` | the impl's, given `f`'s options |
| `Write::write_str(f, s)` of a `Formatter` | `s`, as `f.write_str(s)` is |

- **The impl asks its `Formatter`** for what the placeholder said:
  `f.alternate()` of `{:#x}`, `f.pad(..)` by `{:>10b}`'s width (ADR 0143).
- **Neither trait but `Pointer` has a diagnostic item:** each is known by
  its name in `core`'s `fmt`, as `Sum` is (ADR 0160).
- **Still errors:** `{:p}` of a reference, whose address JS hasn't, and
  `LowerHex::fmt(&n, f)` of a number given a `Formatter`, which would pad
  by its options as it runs.

## Why

- **It's exact:** the `user_fmt_traits` corpus case compares each trait's
  placeholder, `{:#x}`'s alternate form, a width the impl pads by, an impl
  delegating to another, and `write_str` through the trait with native
  Rust.
- **It's the JS a person writes:** the type's function, called as its
  `Display`'s is.
