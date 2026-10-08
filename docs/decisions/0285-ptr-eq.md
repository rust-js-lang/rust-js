# 0285. `std::ptr::eq` of JS objects is whether they're one

Status: Accepted. Extends [0111](0111-js-types-as-structs.md).

Case: A, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`std::ptr::eq(a, b)` asks whether two references are to one place. A port
asks it of JS objects, whether a message came from this frame's window or
an element is the one it holds, and it was an error: the js crate told a
program to use `js::object::is` instead.

A JS object a binding gives, a struct of a `PhantomData<JsObject>` (ADR
0111), is only ever held by reference, and a reference to it is the JS
object: two are of one place exactly when they're one object. A Rust value
isn't so:

- **An unchanged copy is the object it was copied from**: `let b = a;` of
  a `Copy` struct no one changes is `const b = a;`, so `ptr::eq(&a, &b)`,
  `false` in Rust, would be `a === b`, `true`.
- **A number is its value**: two `&u32` of equal numbers at two places are
  `===`.

TypeScript has `===` and nothing of places; ReScript has `===` of any two
values, physical equality, with the same caveat for its unboxed values.

## Decision

**`std::ptr::eq(a, b)` of references to JS objects is `a === b`.** Of any
other type it's an error that says why: rust-js shares a copy no one
changes and writes a number as its value, so where a Rust value is has no
JS counterpart.

```rust
std::ptr::eq(a, b)  // a, b: &webapi::Element
```

```js
a === b
```

- The `*const T` rustc makes of each `&T`, `&raw const *r`, is `r`, the
  object; `r as *const T` is too.

## Why

- **Faithful where it can be**: for a JS object the answer is Rust's, and
  for a Rust value no answer would be.
- **An error over a wrong answer**: a program that compares places of
  Rust values learns so where it's written, not at run time.

## Alternatives

- **`===` of anything**, as ReScript's physical equality: `true` of two
  copies Rust holds apart.
- **Keeping every copy apart** so that addresses mean something: a copy
  of each value passed or bound, to answer a question few programs ask.

## Consequences

- `js::object::is` remains, for two JS objects seen as two Rust types.
- `Rc::ptr_eq` and `ptr::eq` of a `dyn` or a slice are still errors.
