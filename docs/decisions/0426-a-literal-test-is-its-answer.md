# 0426. A test or a value written out of literals is its answer

Status: Accepted. Extends [0066](0066-template-literals.md), whose templates
show values.

Case: A ([0262](0262-when-rust-and-js-disagree.md)): what JS gives is the
same, written as a person writes it.

## Context

What rust-js writes of a constant, a fieldless type's ordering or a
checked shift by a known amount is often a test of two literals:

```js
console.log(`Equal ${0 < 0} ${0 <= 0}`);
tagged.sort((a, b) => 0 || $cmp(a.n, b.n));
const value = 64 < 64 ? BigInt.asUintN(64, 1n << 0n) : undefined;
if (!true) {
  return 1;
}
```

A person writes the answer: `"Equal false true"`, `$cmp(a.n, b.n)`, `None`.

## Decision

**What JS would make of literals alone is written as its answer, where the
expression is built (`js::Expr`), as its other simplifications are:**

- **A comparison of two numbers,** `0 < 0`, `2 === 2`, is `false` or
  `true`.
- **`!` of `true` or `false`** is the other.
- **`&&` and `||` of `true` or `false` first** are what JS gives: `true &&
  x` and `false || x` are `x`; `false && x` and `true || x` the literal,
  where JS never runs `x` either.
- **`0 || x` is `x`**: an ordering's `Equal` part, before the next.
- **`true ? a : b` is `a`,** the branch that runs; `false ? a : b`, `b`.
- **A template shows a `true`, `false` or whole number in its text**, as JS
  shows it, and one that shows nothing else is a string:
  `` `${false} ${true}` `` is `"false true"`.

A statement whose test is then a literal, `if (!false) { .. }`, is its
branch, as one was already.

## Why

- **It's the same program:** each answer is JS's own for those literals,
  and nothing that would run is dropped.
- **It reads as a person writes it,** which the generated code must
  (AGENTS.md). Eleven corpus cases, a JSX case and rustc's tests read
  better, none worse.
- **One place:** `Expr::bin`, `Expr::unary`, `Expr::cond` and
  `Expr::template` are how every expression of each kind is made, so every
  path that writes one is simplified, as `a != null && !!a` already was.

## Alternatives

- **A pass over the finished tree:** a second place that knows JS's
  answers, run after what reads the tree has seen the unfolded form.
- **Fold arithmetic too,** `81 + 32`: not here. Its answer may be a number
  rounded as JS rounds it, or a `BigInt`, which needs its type.

## Consequences

- `${-3n < 0n}` stays: a comparison of `BigInt`s isn't folded, as no corpus
  case makes one worth it yet.
- A mutation per rule, each caught by the snapshot of a case that shows it.
