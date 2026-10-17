# 0441. A `()` given a function value is no argument

Status: Accepted. Extends [0313](0313-console-forms.md)
and [0330](0330-undefined-given-to-js.md).

Case: B ([0262](0262-when-rust-and-js-disagree.md)): only a function
that reads `arguments.length` tells `f()` from `f(undefined)`, and a `()`
is given where Rust's type is `Fn(())`, a resolver's or a callback's,
which none reads it of.

## Context

A `()` is `undefined` in JS. A trailing `undefined` after another
argument is no argument (ADR 0313), but one given alone is a value,
`setProgram(undefined)` of a `None`. A `()` given alone was written the
same way, so a promise of nothing resolved as `resolve(undefined)` where a
person writes `resolve()`: the countdown's `sleep`, once it used the js
crate's `Promise` (ADR 0440).

## Decision

**A trailing argument of type `()` given a function value, a closure, a
`dyn Fn` or a generic `F: Fn`, is left out**: `resolve()`, `f()`, `g(1)`
of `g(1, ())`. A `None` alone is still `undefined`, a value. One whose
making does something is still made, first: `k(noted())` is `noted();
k();`. `calls::units_left_out` says which, of the call's argument types.

## Why

- **It's the JS a person writes**: a call of nothing has no argument.
- **It's what Rust says**: `()` is no value, where `None` is one.
- `unit_arguments` compares a closure's, a `dyn Fn`'s, one after another
  argument, one made by a call and a generic `T` of `()` with native Rust.

## Consequences

- A JS function given a `()` that counts its arguments sees one fewer.
