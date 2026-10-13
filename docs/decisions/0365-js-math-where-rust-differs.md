# 0365. JS's Math where Rust's methods differ: `js::math`

Status: Accepted. Extends [0283](0283-js-crate-covers-typescripts-es-library.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

[ADR 0283](0283-js-crate-covers-typescripts-es-library.md) left `Math` out
of the js crate: `f64`'s methods are it, `x.sqrt()` is `Math.sqrt(x)`. For
most they agree. For a few they don't, and rust-js keeps Rust's answer
(case C), with a helper: `x.round()` takes a half away from zero,
`$round(x)`, where `Math.round` takes it up; `a.max(b)` is the other
number of a `NaN`, `$f64Max(a, b)`, where `Math.max` is `NaN`; and
`x.signum()` is `1` of `0`, where `Math.sign` is `0`.

A program that wants JS's answer had no way to ask for it. react.dev's
home page calls `Math.round`, `Math.max` and `Math.min`; its port wrote
`f64::round`, and its JS read `$round(..)`, with another answer at a
negative half.

## Decision

**`js::math` binds `Math`'s functions whose answer isn't the `f64`
method's, and `Math.random`, which Rust has none of. A program that
wants JS's answer calls them; `f64`'s methods keep Rust's.**

- `round(x)`, `sign(x)`, `random()`: `Math.round(x)`, `Math.sign(x)`,
  `Math.random()`.
- `max(&[a, b])`, `min(&[a, b])`: variadic, `Math.max(a, b)`, as
  TypeScript's `max(...values)` takes any count. ReScript's Stdlib has
  `max(a, b)` and `maxMany(array)`; one variadic spell does both.
- The rest of `Math` stays `f64`'s methods, which are `Math`'s already:
  binding them twice would be two spells for one job.

## Why

- **A port reads as the original**: `Math.round(x)` where it wrote
  `Math.round(x)`, with the original's answer.
- **Nothing changes for Rust code**: `x.round()` is still Rust's, so a
  program's results don't move under it (ADR 0262, case C).

## Alternatives

- **Accept the helpers in ports**: the JS reads unlike the original, and
  its answer differs at a negative half, a `NaN` and `-0`.
- **Write `f64`'s methods as `Math` where they can be shown to agree**:
  a proof per call site of no `NaN`, `-0` or half reaching it, which
  rarely holds, and a program still couldn't ask for JS's.
- **All of `Math`**: `sqrt`, `sin` and the rest would be there twice.

## Consequences

- `test/compiler.test.ts` holds each against Rust's method: a negative
  half, a `NaN`, `-0`, and `sign` of a zero.
- The coverage ratchet doesn't measure `Math` (ADR 0283's scope); these
  are what a program can't reach otherwise.

## Since

2026-10-13.
