# 0612. Two values of any shape are equal by JS's `===`

Status: Accepted; amended (two `Option`s, below). Extends
[0225](0225-unknown-values.md)'s `js::Unknown`.

Case ([0262](0262-when-rust-and-js-disagree.md)): C, results would change:
JS's answer for two values, as a value of any shape is JS's alone; Rust's
for two `Option`s of them, paid for in output, `?? null`.

## Context

React's `shared` code compares `mixed` values, anything at all, by `===`:
its `Object.is` polyfill's `x === y`, where `null` isn't `undefined`. A
`js::Unknown` had no `==`, and an `Option<&Unknown>`, which a value that
may be `null` or `undefined` is, compared by `==`, both `None`.

## Decision

The user's choice: **`impl PartialEq for js::Unknown` is `a === b`**, the
same object or the same value, `NaN` not itself, so it's `PartialEq`
alone, as an `f64`'s. A value of any shape has no meaning but JS's
(`declarations::any_shape`, `js_eq`).

## Amended: two `Option`s are std's

The decision was first that `==` of two `Option<&Unknown>` is `===` of
the JS values too, `null == undefined` `false`, where std's `Option`
calls both `None` equal. That is a wrong answer: the program is rustc's,
whose `Option`'s derived `==` says two `None`s are equal, and a `None`
may be either, as `js::classify` of one is `Kind::Null` or
`Kind::Undefined` (ADR 0676). React's `ReactPerformanceTrackProperties`
port compared two props read by `js::get` so, `prev_value != next_value`,
which was `prevValue !== nextValue`, a change of a `null` prop to a
missing one where Rust sees none.

**`==` of two `Option<&Unknown>` is std's**: both `None`, whichever of
`null` and `undefined` each is, or `Some`s `===` finds equal. In JS,
`(a ?? null) === (b ?? null)` (`js::nullish_equal`): `?? null` makes a
`null` of an `undefined` and leaves every other value as it is, so each
is read once, in order, with no temporary, and `!=` is `!==` of the same.
`==` can't be it, as it makes `"1"` of `1`; nor `Object.is`, whose `null`
isn't `undefined` either; and `a === b || (a == null && b == null)` reads
each twice.

Where the program can't tell `===` alone from it, `===` alone is
written, as a person writes it:

- **One side a `Some`**, a local only ever `Some(..)`, `a == Some(b)`
  (`strictly_equal`, ADR 0030): what's there is never `null` nor
  `undefined`, so `a === b` answers the same; `js::strict_equality` takes
  the `?? null`s away.
- **One side a constant**: `== None` is `a == undefined`, and a value,
  `Some(js::unknown("…"))`, `a === "…"`, as before.

Two `Option`s from the same source still may differ in which `None`
each is: a binding's, a parameter's or a property's `None` is either,
and only a `None` the program makes is always `undefined`. No analysis
tells which a body's `None`s are, so where neither side is known, the
`?? null`s are written.

**A struct, a tuple, a slice or an `Option` holding a value of any
shape compares it part by part**, each by its own `==`, never by `$eq`,
which compared what's in two objects, so two objects alike were equal
(`custom_eq`).

What a port that wants JS's `===` of two values that may be `null` or
`undefined` writes, React's `Object.is` polyfill's, is
`js::strict_equals(a, b)` ([0721](0721-strict-equals-is-jss-triple-equals.md)).

## Consequences

- `None == None` of `Option<&Unknown>` is `true`, a `null`'s and an
  `undefined`'s too, as std's `Option` has it.
- No corpus case's or snapshot's JS changed; the new case
  `unknown_option_equality` runs the mix against native Rust.

## What holds it

- `test/lowering-bindings.test.ts`, "js::Unknown's == is ===, of an
  Option's Somes"; `test/corpus/unknown_option_equality.rs`, `null` and
  `undefined` from bindings, in a slice, a tuple, an `Option` and a
  struct, against native Rust.
- Mutations: `==` of the `Option`s `===` (`option-unknown-strict`) or `==`
  (`unknown-option-eq-loose`, `nullish-equal-loose`); a `Some`'s `?? null`
  kept (`nullish-equal-of-some-kept`); a value of any shape compared by
  `$eq` (`unknown-compared-by-contents`).
