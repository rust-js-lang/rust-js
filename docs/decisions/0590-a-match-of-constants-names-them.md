# 0590. A `match` of constants tests each by its name, its comments in its arms

Status: Accepted. Extends [0014](0014-match-lowering.md)'s `if` chains and
[0444](0444-a-comment-is-where-its-code-is.md)'s comments. Amended: a
comment above an arm of a block, or a guarded arm that does nothing, is
above its `case` (rd619, the user's decision).

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

## Amendment: a comment above an arm of one pattern that opens a block is before its `case` (rd166, 2026-10-31)

rd214 put a comment above a `_` arm that opens a block before its
`default:`, as the program could have written it in the block and didn't;
a case of one value kept its comment in its body. React DOM writes such
comments above its cases, `// case 'title': //We assume all titles are
matchable ..` above `case 'meta': {` (canHydrateInstance) and `// Track
the input node that has focus.` above `case 'focusin':`, and their ports
above the arm, `"meta" => {`; the port printed them after `case "meta":`,
merged with the block's own first comment. **A comment above an arm of
one pattern that opens a block is before its `case`**, by the same reading
as the `_`'s; one inside the block, before its first statement, stays the
body's, for both: the clause is printed just past the arm's `{`
(`cases::clause_start`, `opened_at`, past the block's own comment lines),
which the comment above is before and the one inside after. Before, a
comment inside a `_` arm's block kept the one above it in the body too.
The scheduler's arms are values, `IMMEDIATE_PRIORITY => -1.0`, whose
comments stay their bodies'; and an arm of only comments, `_ => { //
Unknown event. }`, keeps them in its clause. Case
([0262](0262-when-rust-and-js-disagree.md)): N, comments. Held by the
corpus case `switch_guard_skips_its_case` (`// Track the input node ..`),
`lowering-comments-inside`'s "a comment above a _ arm that opens a block
is before its default", and the mutations `block-case-comment-in-body`
(`to_oxc/statements.ts`) and `block-case-start-unlisted`
(`to_oxc/cases.ts`).

## Amendment: a `&str` constant is named too (2026-11-02)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, a constant and its
text test the same: `===` of strings compares their text.

React Flight's clients test a chunk's status by its port's constants,
`match status { Some(INITIALIZED) => .., Some(ERRORED) => .. }`, of
`const INITIALIZED: &str = "fulfilled";`; upstream's `case INITIALIZED:`
was the port's `case "fulfilled":`. MIR tests a string pattern by a call
of `<str as PartialEq>::eq` of its text, not a `switchInt` this ADR
named, so nothing asked `ConstantPatterns` of it, though it recorded the
constant.

**A text a pattern names by a `const` is tested by its name**
(`named_constants::named_text`, of `ConstantPatterns::named`'s
`PatternValue::Text`), as a number's is: `case INITIALIZED:`. Only of the
test MIR makes of a pattern, the call written at a pattern's span
(`ConstantPatterns::pattern_at`): a `==` the program writes in an arm,
`other == "fulfilled"`, is in the `match` too, and keeps its literal.

**To tidy it's its literal, named once tidy's done**: the test compares
with the literal placed at the pattern, and the body records the `const`
read of it (`Written::named_texts`), which tidy's last pass, `named texts` (`named_constants::name_texts`),
puts in each copy of it tidy left, a `switch`'s `case` too. Tidy's
rewrites read a literal as a value never `null` and tell two apart by
their text: `if (status != null) { if (status === "a") .. }` is `status
=== "a"`, and an or-pattern's nested `!==` tests are one `||` (the
`decisions` pass's sets of values). Named before tidy, the first attempt,
a `&str` constant's read kept `status != null &&`, a `switch`'s `default`
after it, and a labeled block for an or-pattern's arms
(`presence_said_by_chain`); marking the read text and accepting it in
each rewrite would have been one more answer beside the literal's in
every pass that reads one. A number's constant needs none of this: MIR
tests it by a `switchInt`, which lowering names where it makes the test.

Held by the corpus case `str_const_patterns` (a plain `match`, of an
`Option<&str>`, of `as_deref()`, `matches!`, `let … else`, a `==` in an
arm, an or-pattern with `None`, a text written two ways in one `match`,
another module's constant, imported) and `presence_said_by_chain` (`value
=== A || value === B`), run against native Rust; and the mutations
`text-pattern-constant-unnamed` (`std_impls.ts`),
`text-named-past-its-patterns` (`body_queries/constants.ts`),
`named-text-left-its-literal`, `named-text-before-tidy` and
`named-text-of-a-written-eq` (`named_constants.ts`).

## Amendment: a comment above an arm of a block, or a guarded arm that does nothing, is above its `case` (rd619, 2026-11-02, the user's decision)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments.

rd166 put a comment above an arm of one pattern that opens a block before
its `case`, by where the arm's first statement is: just past its `{`
(`cases::clause_start`). A first statement whose span starts after code on
its line, `DOWN.set(true)`'s at `set`, had no such place, and React DOM's
SelectEventPlugin printed `case "mousedown": // Don't fire ..`; a comment
above a guarded arm that does nothing, `"selectionchange" if skip => {}`,
went to the guard's `if (skip)` in the case. Upstream writes both above
the `case`. The user decided: **a comment above an arm prints above its
`case` label, as upstream writes it**. A case is printed where its first
value is written (`cases::at_its_value`, as rd143's case of more than one
value and rd136's of an arm that does nothing were) where its arm opens a
block or is guarded and does nothing (`cases::opens_or_guards`, of the
source between the pattern and its `=>` and after), and that start is a
case's (`cases::case_starts`), so the comment is before the `case`, not
the first statement on the guard's line. A comment inside the block,
before its first statement, stays the body's.

Two arms keep their comments in their cases, as upstream does: an arm
that's a value, `IMMEDIATE => -1.0`, the scheduler's (`case
ImmediatePriority: // Times out immediately`), where the arm has no other
place for its body's comment; and a guarded arm that does something,
SimpleEventPlugin's `"keypress" if char_code(..) == 0 => return None`
(`case 'keypress': // Firefox creates .. if (getEventCharCode(..) === 0)`)
and completeWork's `THROW if !DISABLE_LEGACY_MODE => return None`, whose
comment is before the guard's `if`. An arm of only comments, `3 => { //
.. }`, keeps them in its clause, where it ends (`commented_clause`).

The `if` chain the port printed instead, `name === // Chrome and IE ..
"selectionchange" && !skip || ..`, its comment on the test's operand, was
the `decisions` pass's chain of the `match`, whose guarded arm and a
drop MIR decides at the block's end, of a value one arm moves, made a tree
of it: see ADR 0364's amendment of the same day. The `switch` is made again, and its comments
are its cases'.

Held by the corpus cases `switch_guard_skips_its_case` (above `case
"mousedown":` and `case "selectionchange":`), `switch_arm_tests_its_own`
and `switch_text_guard_falls` (the guarded `return`'s comment in its
case), run against native Rust; `lowering-comments-ends`' "a last arm of
only comments keeps them in its clause"; the guide's basics.md; and the
mutations `block-case-start-unlisted` and `stacked-case-starts-of-one-pattern`
(retargeted), `block-case-comment-in-body` (moved from
`to_oxc/statements.ts`; it holds rd136's `"value" => {}` too, an arm of a
block, so `empty-arm-case-at-no-value` and the rule of a body of nothing
written it held are gone), `guarded-nothing-case-comment-in-body`,
`guarded-doing-case-comment-above` and `commented-arm-written-apart`
(`to_oxc/cases.ts`). What the case's start being its value's left to
other tests: `lowering-comments-inside`'s "a comment above an arm of a
block of an if chain is before its first statement" holds
`arm-opening-not-an-opening`, and `lowering-comments-jumps`' "a comment
above a _ arm whose pattern's comment has brackets is before its default"
holds `comment-between-read-apart` (`to_oxc.ts`).
