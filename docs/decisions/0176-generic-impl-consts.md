# 0176. A generic impl's constant of its parameters is its dictionary's getter

Status: Accepted. Extends [0106](0106-generic-traits.md) and
[0031](0031-consts.md).

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

ADR 0106 left out a generic impl's constant made of its parameters':

```rust
impl<T: ConstZero> ConstZero for Wrapping<T> {
    const ZERO: Self = Wrapping(T::ZERO);
}
```

rustc computes a constant's value for the program to use, but this one has
no one value: it's `T`'s, for each `T`. num-traits has two, which stopped
it, and chrono and rust_decimal with it.

## Decision

**A generic impl's constant rustc can't compute is a getter of the impl's
dictionary, its initializer lowered there, where the impl's evidence is:**

```js
get ZERO() {
  return [TConstZero.ZERO];
}
```

- **Read each time, as a constant is:** each use of a Rust constant is a
  value of its own, so a getter is, where a value changed in place could
  otherwise be shared (ADR 0031).
- **Only where rustc can't compute it:** one of no parameter, or of known
  types, is the value rustc computed, as before.
- **Still an error:** a trait's default constant of `Self` the generic impl
  keeps, which would need its `Self` read as the impl's type.

## Why

- **It's exact:** the `generic_impl_consts` corpus case reads a
  `Wrapping(T::ZERO)` and an `A::BITS + B::BITS`, nested and through generic
  code, with native Rust.
- **It's the JS a person writes:** the value, worked out from the
  parameters' own, where it's asked for.

## Amendment: a trait's default, copied into the impl

A trait's default constant of `Self`, `const B: u32 = Self::A + 1`, that
rustc can't compute for a generic impl is its initializer copied into the
impl, as a default method is (ADR 0049), `Self` the impl's type: a getter
of the dictionary. In a library every trait default is, never computed,
as a consumer may never read it and rustc reports one it can't compute,
`360 / Self::A` of 0, only where it's read; one that's a literal is the
field, `NAME: "shape"`.

A default's own initializer isn't generic code reading the constants it
names: its `Self::B` is read only where it's lowered, a library's
dictionary, which has every constant. Counted as one, a program computed
`()`'s defaults of each other, `A = Self::B` and `B = Self::A`, which
rustc errs on only where one is read (its `defaults-cyclic-pass`).

## Amendment: a constant whose value rustc can't say

A trait impl's constant was its value, as rustc computed it, unless its
impl was generic; but rustc's value tree can't say a `String`'s or a
`Vec`'s, `const EMPTY: String = String::new()`, and with no initializer
kept the dictionary was refused (#156). Every trait impl's constant keeps
its initializer, which the dictionary lowers where the value can't be
read: `{ EMPTY: "" }`. Held by `test/corpus/impl_const_string.rs`.

An initializer is kept only where the constant has a body: a `type const`
has none, and lowering one crashed. Held by `test/diagnostics.test.ts`'s
type-constant test.

## Amendment: a library's impl a consumer compiles has its constants (#468)

A consumer compiles a library's trait impl itself where the library's JS
has none it can call (ADR 0914, amended), and since c3ab99271 rust_decimal
compiled arrayvec's `impl<T, const CAP: usize> ArrayVecImpl for
ArrayVec<T, CAP>`. Its `const CAPACITY: usize = CAP;` is a getter of the
dictionary, its initializer lowered there, but only a body of the crate's
own had one: rust_decimal was refused at "a generic impl's constant of its
parameters". A program, which writes only the constants generic code reads,
read them of its own THIR alone, so its dictionary had no `CAPACITY` at
all, and `Self::CAPACITY - len` of the code it compiled was `NaN >>> 0`: a
`Stack<Loud, 2>` full at once.

**An impl a consumer compiles is compiled with its constants, each of the
MIR its library's metadata keeps of it, the one rustc evaluates it from
(`mir_for_ctfe`), as a function's is of its optimized MIR; and the
constants the code it compiles reads, of its MIR, are those its
dictionaries have:**

```js
function arrayVecArrayVecImpl(CAP, dropT) {
  return $traitImpl($arrayVecArrayVecImpl, [CAP, dropT], () => ({
    get CAPACITY() {
      return CAP;
    },
```

- Case N ([0262](0262-when-rust-and-js-disagree.md)): the getter is the
  library's own, as its JS writes it, read where generic code asks.
- `instances::asked` places an impl with its constants beside it, as with
  its methods; `analysis::foreign_bodies` reads each's MIR
  (`foreign_mir`); `analysis::generic_consts` reads the constants a body
  of MIR alone names, `Const::Unevaluated` of a trait's constant at
  generic arguments, as it reads a THIR's `NamedConst`.
- Not rustc's value at the instance's arguments, `const_eval_resolve`:
  the dictionary is compiled once, a function of `CAP`, and called with
  each caller's, so there are no concrete arguments to evaluate it at. One
  rustc can compute, of no parameter, is its value as before.

### What holds it

- `test/crates/triples/generic_impl_const_of_instance`, against native
  Rust: `mid`, a library, compiles `dep`'s `Stack<(), 2>`'s impl, as
  rust_decimal does arrayvec's; `app`, a program, its `Stack<Loud, 2>`'s.
- The crate corpus's rust_decimal probe, which reads
  `Decimal::from_str_exact(..).to_string()` now: runs, the same as native.
- Mutations `instance-impl-consts-unasked` (`lower/instances.ts`),
  `foreign-const-without-mir` and `generic-consts-of-instances-unread`
  (`lower/analysis.ts`).
