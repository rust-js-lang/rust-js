# 0866. A `let` only the `if let` that takes it apart reads is read there

Status: Accepted. Extends [0842](0842-a-let-chain-after-a-test-of-its-own-reads-its-place-once.md)'s
and [0780](0780-an-or-patterns-tests-are-as-written-of-a-field-read-in-place.md)'s
places read in a test to a `let` the program names, of a `RefCell`'s value
too, which [0863](0863-a-refcell-of-an-object-is-the-object.md) made the
object.

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same value read:
nothing runs between the `let` and the test's reads, and the test runs no
code, so the place holds what the `let` read. A read the test's `&&` skips
is of a property no getter answers, which does nothing.

## Context

React's `startTransition` checks, in its `finally`, that an inner
Transition took the outer one's types:

```js
if (__DEV__) {
  if (
    prevTransition.types !== null &&
    prevTransition.types !== currentTransition.types
  ) {
    console.error(..);
  }
}
```

Its port ends the `RefCell`'s borrow before it takes the value apart, as
Rust must where code that may borrow the cell runs while it's held:

```rust
let prev_types = prev_transition.types.borrow().clone();
if __DEV__
    && let Some(prev_types) = prev_types
    && !Rc::ptr_eq(&prev_types, &current_types)
{
    console::error(..);
}
```

which was `const prevTypes = prevTransition.types; if (__DEV__ && prevTypes
!= null && prevTypes !== currentTypes)`. Without the cell, `if __DEV__ &&
let Some(prev_types) = &prev_transition.types && ..`, MIR's temporary of
the place is read in place (ADR 0672), `prevTransition.types != null &&
prevTransition.types !== currentTypes`, upstream's. The `let` the borrow
asks for kept the `const`: tidy reads in place only what MIR made, never a
name the program wrote.

## Decision

**A `let` an `if let` takes apart, the variable alone its scrutinee, whose
value is a place's read, which only the test of the `if` right after it
reads, is that place, read where the test reads it** (`moves::tested_reads`,
after `and tests` makes the chain one test):

- **taken apart**: THIR says so of the `let` (`LetNames::taken_apart`),
  `let x = ..; if let Some(x) = x`: the `let` only holds the value for
  the pattern, as a borrow or a temporary's life asked, which JS hasn't;
- **a place's read**: a property of a variable, a chain of them, which no
  getter may answer (`Expr::has_effects`);
- **only the test reads it**, once or more, the `if`'s branches and what
  follows reading none of it;
- **the test runs no code** (`Expr::has_effects`), so nothing between the
  `let` and each read can change the place.

```js
if (__DEV__ && prevTransition.types != null && prevTransition.types !== currentTypes) {
  console.error(..);
}
```

A `let` the body reads too stays, one read before the test: the body may
run code that changes the place, `console.error` above, after which the
place holds another value than Rust's variable. So `startTransition`'s
`const currentTypes = currentTransition.types;`, which the body writes to
the outer Transition, where upstream reads `currentTransition.types` again.
A `let` no `if let` takes apart, compared or tested, keeps its name, as
React's `const objectPrototype = Object.prototype; if (object == null ||
object === objectPrototype)` keeps its own.

## Why

- **It's the Rust without the borrow**: the `let` is there for Rust's
  borrow check, and the JS reads as the scrutinee written in place reads.
- **The same value**: nothing runs between, as the test runs no code.
- **Names the program writes for themselves stay**: only a `let` an `if
  let` consumes, whose name the pattern binds again, is let go.

## Alternatives

- **Every `let` of a place's read only a test reads**: React's
  `ReactSerializationErrors`' `objectPrototype`, upstream's own name too,
  was lost.
- **A `RefCell`'s value alone**: the same `let` of a plain field's read,
  `let t = &p.plain; if let Some(t) = t`, would read otherwise than the
  `RefCell`'s, and lowering would have to carry which read was a borrow's.
- **Read it in the body too**: a wrong value where the body's code changes
  the cell first.

## Consequences

- No corpus case, snapshot or guide example but the new case changed:
  such a `let` was only React's.
- A test that runs code keeps the `const` before it, as Rust reads the
  place first: `const types = t.types; if (emptied(t) && types != null)`.

## What holds it

- Corpus case `refcell_let_taken_apart` against native Rust: React's
  check, read in the test; one whose test empties the cell first, one
  whose body reads it, and one no `if let` takes apart, each a `const`.
- Mutations `tested-read-of-any-let`, `tested-read-past-a-call`,
  `tested-read-body-unread` (`lower/mir/tidy/moves.ts`) and
  `let-taken-apart-unseen` (`lower/mir/patterns.ts`).
