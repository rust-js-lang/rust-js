# 0953. Two values' truths joined are the truth of the two

Status: Accepted. Extends [0298](0298-truthy-options.md)'s `!!o`, a
value's truth, to two joined by `&&` or `||`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. `!!a && !!b` and
`!!(a && b)` give the same `bool` and read `a`, then `b` only where `a` is
truthy, alike; `||` as well.

## Context

React's `shouldConstruct`:

```js
return !!(prototype && prototype.isReactComponent);
```

Its port, `js::truthy(prototype) && js::truthy(js::get(prototype,
"isReactComponent"))`, wrote `!!prototype && !!prototype.isReactComponent`.

## Decision

**`!!a && !!b` is `!!(a && b)`, and `!!a || !!b` is `!!(a || b)`**, of
each such chain (`prepare::expr`, after its parts).

## What holds it

- `test/lowering-option-truth.test.ts`, "two values' truths joined are
  the truth of the two".
- Mutations: `truths-joined-apart`, `truths-joined-by-and`.

## Amendment: a `bool` binding's static is a `boolean` (2026-10-27)

React's `supportsSingletons && fiber.tag === HostSingleton`, its port's
`extern` static `bool`, read `!!supportsSingletons && ..`: a binding's
static of type `bool` is `Known::Boolean`, as a `bool` constant read by
its name is (ADR 0031, amended), TypeScript's `boolean`, so `&&` gives it
as it is. Held by the test of ADR 0947's amendment and mutation
`bool-binding-static-not-boolean`.
