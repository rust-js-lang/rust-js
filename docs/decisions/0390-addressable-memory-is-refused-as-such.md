# 0390. Addressable memory is refused, and says so

Status: Accepted, amended by [0406](0406-a-union-is-the-field-it-holds.md):
a union isn't addressable memory; reading its bytes as another type's is,
refused where it runs. Extends [0262](0262-when-rust-and-js-disagree.md).

Case: D ([0262](0262-when-rust-and-js-disagree.md)): JavaScript has no
memory a program can address, so what reads or writes it can't be kept.

## Context

About 106 of rustc's in-scope tests that rust-js rejects use addressable
memory: a raw pointer, `transmute`, `mem::zeroed`, a union, `std::ptr`'s
reads and writes. Each was refused in words of its own part, "values of
type `*const isize`", "calling `std::intrinsics::transmute`", "unions",
which said what was refused but not why, nor that these are one feature
rust-js doesn't have. The words were written in eleven places, each its
own `format!("rust-js does not support {what} yet")`.

## Decision

**What's of addressable memory is refused as such: "rust-js does not
support addressable memory yet: ..", with a note of why. Every refusal is
said by `refusal.rs`, one way.**

```
error: rust-js does not support addressable memory yet: calling `std::intrinsics::transmute`
  = note: JavaScript has no memory to address: a raw pointer, `transmute` and `std::ptr` read or write bytes by where they are (ADR 0390)
```

- **`refuse(tcx, span, Refused)`** makes every refusal, and `words` says
  it: a feature, a feature and why, a call (`calling`), a type's values,
  constants or statics, or a feature of memory.
- **Which are of memory is recognition's**, which knows std: a call std makes
  for it, `std::ptr`'s and a raw pointer's own, `transmute`,
  `transmute_copy`, `zeroed` and `uninitialized` (`addresses_memory`); a
  type with a raw pointer, a union or a `NonNull` in it, in a crate's own
  type's fields too, but not in std's, whose private fields a `Vec`'s
  pointer is among (`is_memory_type`); and a static's raw address, a raw
  pointer to a value, `&raw const x` or `&x as *const T`, and a union's
  field, refused where they're made or read.
- **Only what's refused is asked**: `ptr::eq` (ADR 0285), `MaybeUninit`
  (ADR 0332) and a `static mut`'s place are supported as before.
- **The architecture test holds the one way**: no other file says "rust-js
  does not support".

## Why

- **It says the feature and why**: a person, or an agent, reads that it's
  memory rust-js lacks, not a type or a function to work around.
- **It's tested**: the diagnostics test refuses a transmute so;
  `union_const.rs` and `static_raw_address.rs` expect the words; their
  mutations, and each refusal's, are caught.

## Consequences

- rustc's tests of addressable memory stay listed as known failures,
  rejected clearly, with these words.
