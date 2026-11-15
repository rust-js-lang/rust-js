# 0957. A place a test shows isn't `null`, stored right after it, is stored as it is

Status: Accepted. Extends [0620](0620-an-option-a-test-says-is-there.md),
`?? null` decided by an enclosing test of a variable, to a place's, a
field read, of a part of the test's `&&`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Where the test held,
the place is neither `null` nor `undefined`, so `p ?? null` is `p`.

## Context

React's `popHydrationStateOnInterruptedWork`:

```js
if (fiber.tag === HostComponent && fiber.stateNode != null) {
  nextHydratableInstance = fiber.stateNode;
}
```

Its port's `NEXT_HYDRATABLE_INSTANCE.set(fiber.state_node.get())` of a
nullable thread-local was `nextHydratableInstance = fiber.stateNode ??
null`: `nulled_as_they_are` took a test's word only of a variable it
showed.

## Decision

**An `if` whose test, or a part of its `&&`, is `p != null`, of a place
that reads the same (`reads_same`, no getter), whose first statement makes
`p ?? null` first, a declaration's value or an assignment's of a place
that runs nothing, is `p`**: nothing runs between the test and the read.
A part of an `||` shows nothing, and stays `?? null`.

## What holds it

- `test/lowering-nullable-reads.test.ts`, "a place a test shows present
  is stored as it is".
- Mutations: `present-place-stored-coalesced`, `present-place-of-an-or`.

## Amendment: a `let` set only to values is returned as it is (2026-10-27)

React's `getParentFragmentInstances` returns `let parentFragmentInstances
= null`, set only to arrays. Its port returned `?? null` of it. **A `let`
the body declares once, made and set only to what's never `undefined`, is
a value or `null` wherever it's read** (`lets_never_undefined`): not one a
closure or a function declared in the body mentions, which may set it, nor
one set by a compound or a pattern. Held by
`test/lowering-nullable-reads.test.ts`, "a let set only to values is
returned as it is", and mutations `lets-never-undefined-unread` and
`lets-never-undefined-of-closures`.

## Amendment: a field so shown is not tidy's to say (2026-11-15, #1950)

A field a test shows isn't `null`, `if (fiber.tag === HostComponent &&
fiber.stateNode != null) { nextHydratableInstance = fiber.stateNode; }`,
is read twice in JS, and a struct JS gives, a getter of which may answer
each read anew (rd204, ADR 1022, #1908), may give `undefined` the second
time: stored as it is, `undefined` where Rust stores `null`. That's a wrong
answer, #1950 (P0). Tidy can't tell that struct from the program's own
data in the printed JS, so its rule now takes **a variable alone**, a
module's too, `chunk.value = deferred` after `deferred != null &&
chunk.blocked`, never a read through a property (`Expr::reads_a_field`).
A field so shown is the nulled owner's to prove, of the program's own data
alone (`nulled::field_reads`, ADR 0620, amended, #1933).

Case of [0262](0262-when-rust-and-js-disagree.md): results change, Rust's
kept. React's port, the lead's decision on Tu's behalf: `nextHydratableInstance
= fiber.stateNode ?? null` and `transitionCallbacks =
options.unstable_transitionCallbacks ?? null` (twice), each of a struct JS
gives. Held by test/lowering-nullable-reads.test.ts's "test shows present"
(a getter answering a value, then `undefined`, stores `null`) and
test/lowering-nullable-tests.test.ts's "a variable a part of an && shows
there"; mutations `field-path-shown-present` new, `present-place-stored-coalesced`
and `present-place-of-an-or` retargeted.

## Amendment: a `let` set only to values is MIR's to say (2026-11-16, #1953, #1933)

`lets_never_undefined`, tidy's answer of a `let` made and set only to what's
never `undefined`, is gone: each `let` it answered is MIR's now, a variable
each of whose writes is `null` or a value (`stored_in`, ADR 0620's
amendments of #1933 step 4 (A) and (B)), so its mutations,
`lets-never-undefined-unread` and `lets-never-undefined-of-closures`,
survived on main. Without it, every corpus case, snapshot and React's port
is the same (`check:fast --all`, the port's regeneration), React's
`return parentFragmentInstances;` among them. "a let set only to values is
returned as it is" holds it, of MIR's answer.
