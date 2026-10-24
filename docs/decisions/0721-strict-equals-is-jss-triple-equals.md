# 0721. `js::strict_equals(a, b)` is JS's `===`

Status: Accepted. Extends [0612](0612-unknown-equality-is-strict.md).
Fixed by [0775](0775-a-unit-given-to-js-is-undefined.md): a `()` compared
was `[]`, `x === []`, never true; it's `undefined`, `x === undefined`.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: a binding of JS's own
operator, which is what the program asks for; nothing of Rust's `==`
changes.

## Context

ADR 0612, amended, made `==` of two `Option<&Unknown>` std's: two `None`s
equal, a `null` and an `undefined` too. That is Rust's meaning, but a
port of JS's `===` then had no way to say JS's: React's `Object.is`
polyfill, `x === y`, where `is(null, undefined)` is `false`, ported as
`x == y`, now answered `true`. Telling them apart by `js::classify` of
each is four arms where JS has one operator.

## Decision

The user's decision: **`js::strict_equals(a, b)` is `a === b`**, of any
two JS values, an `Option`'s `None`s as JS has them, `null` not
`undefined`. Each side is `impl IntoUnknown`, what TypeScript's `any` is
in the js crate: a `&Unknown`, an `Option<&Unknown>`, a
`&JsObject`, text or a number, each given as its JS value. Its link name
is `===` (`JsForm::StrictEquals`).

`==` stays Rust's (ADR 0612). A port writes `==` where the Rust means
its values are equal, and `js::strict_equals` where the JS it ports asks
`===` of what may be either `None`. It's the one way a program writes
JS's `===`.

## Alternatives

- **`Option<&T>` of a `Defined` `T` each side**, as `js::truthy` takes:
  a `&Unknown` or a number would need `Some(..)` written around it.
- **`js::classify` of each**: right, and four arms of what JS writes as
  one operator.

## Consequences

- react-in-rust's `objectIs` writes `js::strict_equals(*x, *y)` where it
  wrote `x == y`.

## What holds it

- `test/lowering-bindings.test.ts`, "js::strict_equals is JS's ===, of a
  null and an undefined too".
- Mutations: `===` as `==` (`strict-equals-loose`).
