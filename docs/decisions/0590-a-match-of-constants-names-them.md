# 0590. A `match` of constants tests each by its name, its comments in its arms

Status: Accepted. Extends [0014](0014-match-lowering.md)'s `if` chains and
[0444](0444-a-comment-is-where-its-code-is.md)'s comments.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A name and its value
test the same.

## Context

React's scheduler picks a timeout by its priority:

```rust
let timeout = match priority_level {
    // Times out immediately
    IMMEDIATE_PRIORITY => -1.0,
    // Eventually times out
    USER_BLOCKING_PRIORITY => USER_BLOCKING_PRIORITY_TIMEOUT,
    ..
};
```

MIR's switch tests values, so the JS tested numbers, `priorityLevel ===
1`, and lost the arms' comments: the first went before `let timeout;`,
the next, before an `else if`, which is no statement, nowhere. Upstream's
cases name the constants, each comment in its case.

## Decision

**A value a body's patterns all name by one `const` is tested by its
name** (`body_queries::constant_patterns`, of THIR's patterns'
`expanded_const`): `priorityLevel === ImmediatePriority`, the crate's own
imported where it's another module's, as any read of it is
(`named_const`). One a pattern writes another way too, a literal or
another constant, is its number.

**A comment before a match arm is before its body**: a statement start
whose text to the next is only a pattern and its `=>` is the arm's test,
and the comment goes to the next.

```js
if (priorityLevel === ImmediatePriority) {
  // Times out immediately
  timeout = -1;
} else if (priorityLevel === UserBlockingPriority) {
  // Eventually times out
  timeout = userBlockingPriorityTimeout;
}
```

## Why

- **It's what the program wrote**: the constant names the case; its
  number is how rustc tests it.
- **One answer**: a constant's read is `named_const`'s, in a test as
  anywhere.

## Consequences

- `const_fn_in_patterns`'s tests name its constants.

## What holds it

- `test/lowering.test.ts`, "a match of constants tests each by its name,
  its comments in its arms": its own module's and another's, each arm's
  comment, and a value written two ways a number.
- Mutations: a value written two ways named, the names left out, the
  comment kept at the test.

## Amendment: a trait's constant is its value

A pattern of a trait's constant, `<u8 as Level>::HIGH`, was named by the
trait's item, of no type, and resolving it crashed rustc (rustc's
`associated-consts/associated-const-const-eval.rs`, in the nightly). A
value is named only by a constant of no generics; a trait's is its value,
`case 250:`. Held by `test/corpus/match_trait_const.rs` and the mutation
`pattern-trait-const-named`.

## Amendment: a transparent struct's constant is named too (2026-10-27)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, `===` of the number
is the struct's `==`: a `#[repr(transparent)]` struct is its field in JS
(ADR [0584](0584-a-transparent-struct-is-its-field.md)), and a
constant pattern of it needs the structural `PartialEq` of that field.

React's onCommitRoot, `switch (eventPriority) { case
DiscreteEventPriority: .. case ContinuousEventPriority: .. default: .. }`,
of priorities its port makes `pub struct EventPriority(pub i32)` (c23 of
the port's gaps). Its constants' patterns are rustc's `EventPriority(2)`,
whose value was the struct's, not a number's: the `match` was `if
(eventPriority === 2) { .. } if (eventPriority === 8) { .. }`.

**A transparent struct's pattern's value is its field's**
(`body_queries::transparent_subpattern`, `pattern_value`), named by the
constant as a number's is, its field's pattern not again; and of such a
constant, a `switch`'s case (`hints::valued_constants`): `case
DiscreteEventPriority:`.

Held by `test/lowering-constants.test.ts`, "a match of a transparent
struct's constants names them in a switch", run; the mutations
`transparent-constant-unnamed`, `transparent-constant-field-walked`
(`body_queries.ts`) and `transparent-constant-no-case` (`mir/hints.ts`).

## Amendment: each `match` names its own constants (2026-10-28)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, a constant and its
value test the same.

React's renderRootConcurrent, `switch (workInProgressSuspendedReason) {
.. case SuspendedOnInstanceAndReadyToContinue: { switch
(workInProgress.tag) { case HostHoistable: .. case HostComponent: .. } }
}`, tests two constants of one value, 5, in two `match`es (gap wl7 of the
port). A value was named by the body's patterns together, so the one value
two constants name was neither: `case 5:` and `tag === 5`.

**A value is named by the patterns of the `match` or `let` that tests
it, at the part of what it matches the test reads**
(`body_queries::ConstantPatterns::named`): the innermost written around
the test, a macro's, `matches!`'s, where it's called; and in it, the
patterns whose steps from what's matched, fields and variants, are those of
the test's MIR place from a part of it of the matched type
(`pattern_places`), so `(SUSPENDED_ON_INSTANCE_AND_READY_TO_CONTINUE,
HOST_TEXT)` and `(_, HOST_COMPONENT)` name each field's. One a `match`
writes another way too there, a literal or another constant, is its number
there, and named by its constant in another `match`, as each is written:
`matches!(a, NORMAL) && matches!(b, 3 | 4)` is `a === NORMAL && (b === 3
|| b === 4)`.

Held by `test/lowering-constants.test.ts`, "constants of one value each
name it where they're tested", run: nested `match`es, a tuple's fields,
a tuple in a tuple, `if let`, `let … else`, two `matches!`, a value written
two ways in one `match`; and the mutations
`same-valued-constants-of-the-outer-match`, `-of-any-field`,
`-of-any-part`, `-of-another-expansion`, `if-let-constants-unnamed`,
`let-else-constants-unnamed` and `constant-written-two-ways-named`
(`body_queries.ts`).

## Amendment: a comment above a `_` arm that opens a block is before its `default` (2026-10-30, rd214)

A comment above a match arm is its body's (above), which holds of an arm
that's a value, `// Not useful descriptors.` above `_ => String::new()`,
upstream's `default: // Not useful descriptors.`, the Rust having no other
place for it. Above an arm that opens a block, `_ => {`, the program could
have written it in the block and didn't: React's BeginWork `// The default
is now forwards.` above `_ => {`, which stands for upstream's `case
'forwards': default:`, the comment above both labels. **In a `switch`, a
comment above a `_` arm that opens a block is before its `default:`**,
which is printed where its first statement is where the source opens such
an arm there (`Cx::opens_block_default`, `cases::opens_default`'s test,
which a `default` fallen to asked already, ADR 0905, amended); a case of
one value keeps its comment in its body, the scheduler's. Held by
`lowering-comments-inside`'s "a comment above a _ arm that opens a block
is before its default" and the mutation `block-default-comment-in-body`
(`to_oxc/statements.ts`); no corpus case changes.
