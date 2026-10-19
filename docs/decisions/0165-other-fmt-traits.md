# 0165. `{:x}`, `{:e}` and `{:p}` of the crate's types call its own impls

Status: Accepted. Extends [0054](0054-display.md) and
[0143](0143-formatter-options.md).

Case: N, D ([0262](0262-when-rust-and-js-disagree.md)).

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

## Amendment: a number's `{:e}` given by a type's own impl

deranged's ranged integers show their number's `{:e}`,
`fmt::LowerExp::fmt(&self.0, f)`, which was refused: only the radix traits
of a number were std's here. A number's `LowerExp` and `UpperExp` are its
`{:e}` and `{:E}`, `exp_text`, the one place a placeholder's are made too,
padded by the options the writer is given. A precision with an exponent,
`{:.2e}`, rounds as Rust rounds, which isn't made yet: it was refused of a
number, and is now of any type, the crate's own too, as its impl may give
the precision to a number's. `test/corpus/exp_forwarded.rs` shows them
against native Rust; `diagnostics.test.ts` holds the refusal.

