# 0968. Two places set the same value are one assignment, `a = b = v`

Status: Superseded, reverted by the user's decision of 2026-10-27: two
places set the same value are two statements, as written (below).

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference, where the rule below holds.

## Context

React's hooks set two places to one value in one statement:

```js
hook.memoizedState = hook.baseState = initialState;
workInProgressHook = workInProgressHook.next = hook;
newBaseQueueFirst = newBaseQueueLast = clone;
```

Rust has no chained assignment, and its port writes two statements,
`hook.base_state.set(initial_state); hook.memoized_state.set(initial_state);`,
which printed as two: `hook.baseState = initialState; hook.memoizedState =
initialState;`.

## Decision

**`b = v;` followed by `a = v;` is `a = b = v;`** (`prepare/chains.rs`, on
the completed tree), only where it does what the two did:

- **`v` is a variable or a constant**, `initialState`, `0`, read the same
  however often it's read; not a value made of more, `n + 1`, which the
  two statements evaluate twice, nor `b`'s own variable.
- **`a` and `b` are each a variable or a plain field of a path**, `x.y.f`:
  no binding's setter, which may do anything (`Known::Setter`), no getter
  read on the way (`Known::Getter`), no item by index.
- **`a`'s object reads nothing `b` sets.** JS evaluates `a`'s object
  before it runs `b = v`, where the statements read it after:
  `current = node; current.next = node;` stays two, as `current.next =
  current = node` sets the old `current`'s `next`; so does
  `pinned.current = node; pinned.current.next = node;`, and any `a` whose
  object reads a property of `b`'s name, which may be of the same object.

**Two alone**, as React chains them: a run of three or more set alike,
drop flags' `fused$live = false;` each, stays a statement each, which
reads better than a chain of eight across nine lines.

The assignment's value is `ExprKind::Assigned`, extended from a variable's
name to a target, a variable or a field (ADR 0860's loop tests are of a
variable, as before): one form for an assignment used as a value.

The order Rust writes is the order JS runs: the place set first is on the
right. React's `hook.memoizedState = hook.baseState = initialState` is
written `hook.base_state.set(..); hook.memoized_state.set(..);`.

## Alternatives

- **Any two assignments of an equal value**: `n + 1` evaluated once
  instead of twice, a call's effects once; and an alias set through `b`
  read by `a`'s object, a wrong answer.
- **Two statements, as before**: every hook's mount a statement longer
  than upstream's.

## Consequences

- `test/corpus/assignments_chained.rs`, against native Rust and its
  snapshot: a field pair, a constant, a variable and a field, a value of
  more than a name kept apart, two different variables, a run of three kept apart, and the two
  aliasing cases kept apart; `test/lowering-chained.test.ts`, a binding's
  setters kept apart. Mutations `chained-*` and `assigned-target-unwalked`.

## Superseded: two places set the same value are two statements

The user decided (2026-10-27) to revert this rule. Measured on the React
port, `a = b = v` matched upstream at about 15 places and made about 20
others differ, where upstream writes the two statements; and it lost a
comment written between them. Which of the two upstream writes is the
author's style at each place, and no shape of the Rust tells them apart,
so each assignment the program writes is its own statement, as written.

`prepare/chains.rs`, its mutations and `test/lowering-chained.test.ts`
are gone. `ExprKind::Assigned` keeps its target of a variable or a plain
field, ADR 0860's loop tests' form. `test/corpus/assignments_chained.rs`
stays, holding the two statements against native Rust.

`docs/readability.txt` is blessed with it: `renamed x$1` grows, 693.4 to
693.7 in 10,000 lines, as each of a run of drop flags' `a$live = b$live =
false;` is its own statement again, each naming its flag.
