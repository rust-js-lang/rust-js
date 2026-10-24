# 0743. A part of a chain of tests an earlier test decides is left out

Status: Accepted. Extends [0564](0564-a-std-body-is-tidied-as-the-apps-are.md)'s
decided tests to a part of a chain.

Case ([0262](0262-when-rust-and-js-disagree.md)): it can't be observed,
but where it's rare and harmless: a test left out does nothing, and
`Array.isArray` taken as doing nothing differs only for a revoked
`Proxy`, which it throws for.

## Context

React's `getArrayKind` (shared/ReactPerformanceTrackProperties.js) tells
an array's items apart with a `match` of `js::classify`:

```rust
match js::classify(value) {
    Kind::Array(&[first, _]) if matches!(js::classify(first), Kind::String(_)) => { .. }
    Kind::Object(_) | Kind::Array(_) => return ArrayKind::Complex,
    ..
}
```

An array whose pattern fails goes on to the next arm, and one that isn't
an array to `Kind::Object`'s test, `typeof value === "object" && value !==
null && !Array.isArray(value)`. Where that fails, MIR goes on past the
arms, so the JS read:

```js
if (Array.isArray(value)) {
  ..
} else if (typeof value !== "object" || value === null || Array.isArray(value)) {
  break bb12;
}
```

`Array.isArray(value)` is false in that `else`. Tidy's decided tests
(ADR 0564) take a branch whose whole test an earlier one decides; this one
is a part of a chain, and a call, which tidy took to do something.

## Decision

**In a branch an earlier test decides, a part of a chain of `&&` and `||`
that is that test, or its negation, is its value, and the chain is folded
without it**: `a || false` is `a`, `true || b` is `true`, and `&&` the
other way; a known part is left out of its chain only where what's beside
it does nothing (`decided_part`). Not past the `if`, after a branch that
leaves: there a `match`'s arms are tested one after another, each test
whole, which a `switch` of them reads (ADR 0668), and `typeof value ===
"object"` left of `Kind::Object`'s test would be no arm of one.

**`Array.isArray(v)` does nothing but what `v` does**, as `Math`'s
functions and `Symbol.for` don't (`has_effects`): what a value is, which
`Kind::Array` and `Kind::Object` test.

## Consequences

- The guard statements corpus case's `if (s.TAG === "Square" && s._0 >
  2)`, in a branch of `s.TAG === "Square"`, is `if (s._0 > 2)`.
- The port's `getArrayKind` tests `Array.isArray(value)` once. Its `bb12`
  block is MIR's, of the arms that go on past the `match`, still.
- `Array.isArray(x)` of a revoked `Proxy` throws a `TypeError`; one tidy
  now leaves out, or reads once where it was read twice, doesn't. A
  program can't make a revoked `Proxy` without binding `Proxy.revocable`
  itself.
