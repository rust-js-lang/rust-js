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

## Amendment (2026-11-13, #1672 change 1 M1a): a check is left out only through the one node at its span

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so Rust's
behavior is kept. MIR says where a statement is written only by its span,
and the range analysis's facts are of THIR's nodes. The join between them
was loose. A bounds check was left out where *any* THIR index at its span
was known in bounds. A macro's repetition, `[$($v[$i]),*]`, writes one
index at one span for each item, so `v[k]` in bounds left out `v[k + 1]`'s
check, and the JS read past the end where native Rust panics. And a wrap
was left out by a table of spans, ANDed over only the operators the walk
reached.

**Each join that licenses leaving out a check now asks one owner,
`body_queries/span_join.rs`: the one THIR node of a kind at the span, or
none.**
- `known_in_bounds` takes the one index there.
- `exact` is now of operators, `ExprId`s, and MIR's operator is joined by
  `exact_at` to the one written there or at the assignment it's the value
  of.
- `thir_call`, `kept_cast_at` and `closure_arguments_written`, unique
  already, ask it too.

`test/architecture-mut-refs.test.ts` holds the facts `ExprId`-keyed and the
bounds join free of `any`. No snapshot moves. Corpus case
macro_repeated_index_keeps_each_check against native Rust; mutations
`bounds-join-any`, `assigned-operator-span-off` (moved to `mir/bounds.ts`),
`exact-wrapped` and `mir-jsx-shapeless` (refitted), caught.

## Amendment (2026-11-13, #1672 change 1 M1b): the facts that leave out a panic or a test are of nodes too

Case ([0262](0262-when-rust-and-js-disagree.md)): N; nothing a program
observes changes. Six more facts that license leaving out a check were
span sets:
- an `unwrap` whose `None` check goes: `set_then_unwrapped` and
  `getter_unwraps`;
- a value told present, whose `undefined` test goes: `present_calls`;
- a guard that holds, whose test goes: `held_guards`;
- a registry read or comparison, whose `$symbolFor` goes:
  `registered_reads` and `registered_equalities`.

They are now `ExprId` sets, of the call or the guard. MIR's call is joined
to its node by `span_join::the_call_at`, the one call at its `fn_span` or
none. A held guard joins MIR's statements by its span only where no other
guard is written there. No snapshot moves.

`test/architecture-mut-refs.test.ts` holds all ten deletion facts
node-keyed. The body's remaining span sets choose a form, not whether a
check runs, so they are outside this rule: `resumed_as_given`,
`method_calls`, the in-place adapters and the collect sources. Twelve
mutations were refitted to the new joins, all caught.
