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
