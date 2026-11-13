# 0292. An index a condition shows in bounds is read as it is

Status: Accepted. Narrows where `$index` checks an index. Its regions are
replaced by [0576](0576-what-a-bodys-integers-can-be.md)'s ranges, which
know each of its cases and more.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`v[i]` panics past the end, where JS reads `undefined`, so it's
`$index(v, i)`, which throws as Rust panics. Only a constant index of an
array whose length its type says was read as it is. react.dev's Preview
reads the first lint error where it knows there is one:

```ts
if (lintErrors.length === 0) {
  return null;
} else {
  const {line, column, message} = lintErrors[0];
```

Its Rust, `if lint_errors.is_empty() { None } else { .. &lint_errors[0] ..
}`, gave `$index(lintErrors, 0)`: a helper where the original reads the
item.

## Decision

**Of a `Vec` or a slice and an index the function never changes, an index
a condition shows in bounds is read as JS reads it: `xs[0]`.**

What shows it, and where:

- In an `if`'s branch, what its condition shows held, or didn't, there:
  `!xs.is_empty()`, `xs.len() > k`, `xs.len() >= k`, `xs.len() == k`,
  `xs.len() != 0` show the indexes below `k`; `i < xs.len()` shows `i`.
  `!`, `&&` that holds and `||` that doesn't, combine them.
- After `if c { return ..; }`, what `c` not holding shows, to the end of
  its block.
- In `for i in 0..xs.len()`, `i`.

"Never changes": nothing in the function sets it, lends it as `&mut`, or
binds it `ref mut`, in whole or in part. A closure's body is its own: it
knows only what its own conditions show.

## Why

- **It's the JavaScript a person writes**: `lintErrors[0]` after the test.
- **It's the same program**: where the condition showed the index in
  bounds, `$index` could never throw, and nothing changed since.

## Consequences

- Most indexes stay checked: an index a condition doesn't show, as
  `while i < xs.len()` with `i += 1`, is `$index`.

## Amendment (2026-11-13, #1659, #1660): the walk follows Rust's evaluation order

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so Rust's
behavior is kept. Two places where the range walk's order wasn't Rust's
dropped a check:

- **A match arm after a failed guard (#1659).** Each arm started from the
  state before the match, so what an earlier arm's guard changed, `true if
  v.pop().is_none()`, never reached the arms tried after it, and the next
  arm's `v[i]` lost its bounds check. Now each arm starts from the state
  before the match joined with the state after each earlier guard.
- **A compound assignment (#1660).** The walk took the place, then the right
  side, then the place again. Rust evaluates a built-in compound
  assignment's right side first, and THIR keeps an overloaded one apart, as
  a call. So the operator saw the place as it was before the right side
  wrote it: `x += { x = n; 1 }` lost its wrap. And the first walk's index
  narrowed `i` for the right side's own `v[i]`, whose check was dropped. Now
  it's the right side, then the place.

No snapshot moves. Corpus cases
match_guard_that_shrank_a_vec_keeps_the_next_arms_index_checked,
compound_assignment_reads_its_place_after_its_right_side and
compound_assignment_right_side_index_checked_first against native Rust;
mutations `guard-failure-unjoined` and `assign-op-place-first`
(`body_queries/ranges.ts`).
