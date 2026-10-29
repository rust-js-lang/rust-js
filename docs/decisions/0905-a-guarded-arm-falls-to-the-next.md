# 0905. A guarded arm whose constants the next arm names falls through to its case

Status: Accepted. Amends [0636](0636-a-guarded-arm-is-a-case.md), whose
guarded arm was a case only where its failed guard went on to nothing,
and [0629](0629-a-match-of-constants-is-a-switch.md), whose cases each
ended where they went on. Amended below: the last clause has no `break`
tidy ended it with; and a guard that falls to nothing beside one that
falls to the next is a case too, and a case's `if` ends with no `break;`
its end does; and the arm fallen to may be guarded and fall to nothing,
what's past the `match` follows the `switch`, a guard is its case's
`if`, of `||` too.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, nothing the program
can observe. Of each value, the `switch` runs the guards Rust runs, in
Rust's order, each once its constant matched, and then what the arm
Rust takes does: JS's fall-through runs the next case's statements, and
the next case is the arm Rust tries next.

## Context

React's `getComponentNameFromFiber` writes a `switch` whose cases fall
through:

```js
case ViewTransitionComponent:
  if (enableViewTransition) {
    return 'ViewTransition';
  }
// The display name for these tags come from the user-provided type:
// Fallthrough
case IncompleteClassComponent:
case IncompleteFunctionComponent:
  if (disableLegacyMode) {
    break;
  }
// Fallthrough
case ClassComponent:
case FunctionComponent:
  ..
```

Its port writes what Rust has for it, each arm the constants the cases
before fall to, a guard where the case tests one:

```rust
VIEW_TRANSITION_COMPONENT if ENABLE_VIEW_TRANSITION => return ..,
VIEW_TRANSITION_COMPONENT | INCOMPLETE_CLASS_COMPONENT | INCOMPLETE_FUNCTION_COMPONENT
    if DISABLE_LEGACY_MODE => {}
VIEW_TRANSITION_COMPONENT | INCOMPLETE_CLASS_COMPONENT | ..
    | CLASS_COMPONENT | FUNCTION_COMPONENT => { .. }
```

A guard a later arm goes on from wasn't a case (ADR 0636), so the whole
`match`, thirty arms, was an `if` chain nested four deep, `if (tag !==
ClassComponent && tag !== FunctionComponent && ..) { .. }` around the
arms after it, `return null` copied into each.

## Decision

**A guarded arm whose next arm names each constant it does, and one of
its own, falls to that arm: its case is the guard's `if`, and where the
guard fails, it falls through to the next arm's case, `// Fallthrough`
before it.** The next arm's case is the constants it names that the arm
before doesn't; it is guarded too, or not, and is a case as any arm is.

```rust
match tag {
    VIEW_TRANSITION if view_transitions => return Some("ViewTransition".to_string()),
    VIEW_TRANSITION | INCOMPLETE_CLASS if legacy_off => {}
    VIEW_TRANSITION | INCOMPLETE_CLASS | CLASS => return Some(ty.to_string()),
    _ => {}
}
None
```
```js
switch (tag) {
  case VIEW_TRANSITION:
    if (viewTransitions) {
      return "ViewTransition";
    }
  // Fallthrough
  case INCOMPLETE_CLASS:
    if (legacyOff) {
      break;
    }
  // Fallthrough
  case CLASS:
    return ty;
}
```

- **Which arm falls is the hint's**, `hints::written_switches`, by the
  values its constants are (`body_queries::pattern_value`, one walk with
  `constants_of`): each of the arm's is the next arm's, which has one of
  its own. Of a value whose constant isn't told, a string's, a float's,
  a variant's, none falls. `SwitchArm::falls` carries it to tidy.
- **The case is what MIR made, factored**: tidy (`tidy/switches.rs`)
  reads each arm's test as before, an or-pattern's alternatives MIR tests
  apart, each with the guard, one case of their statements where they're
  alike (`stacked`), and an arm tested negated around the arms after it,
  `if (tag !== CLASS) { .. }`, wherever it is in the chain. A falling
  case's statements are its guard's `if` followed by the next case's,
  in one of the forms MIR writes (`guard_alone`): its statements are
  that `if` alone, or the case is no `switch`. So a case falls only where
  its statements, run, are the guard's `if` then the next case's, exactly
  what JS's fall-through runs: whatever MIR made, the `switch` runs it.
- **A falling case only tests on its way to the next**: the guard's
  `if` leaves where the guard holds, a `break;` after the arm's statements
  where they go on, and where it fails has only evaluated the test. So a
  pass that reads a case as starting where the `switch` does reads the
  next one right too; `js::ends` of a `switch` takes a falling case to go
  where the next does.
- **What MIR placed after the chain**, the statements of the arm the
  others fall to, written in it, each case's that went on to them, is
  that arm's case: `case CLASS: ..`, as written. Of a block MIR made of
  them, `bb9: { switch (..) { .. case CLASS: break bb9; } } ..`, a block
  only one case reaches is that case's (`blocks_one_case_reaches`).
- **A `return` MIR copied into a case of what follows the `match`**, the
  `return null` after the `switch`, is the case's `break`, as written:
  `if (disableLegacyMode) { break; }`, of any `switch`.
- **Printing**: a falling case has no `break;` (`js::Case::falls`), and
  `// Fallthrough`, React's word, which ESLint's `no-fallthrough` takes,
  is before the case it falls to.

## Why

- **It's React's `switch`**, line for line, and what a person writes of
  arms that share what they do once a guard fails.
- **Rust's order is the `switch`'s**: a value's case is the first arm
  naming it; its guard fails to the next arm that names it, which is the
  next case. No guard runs twice, or out of turn.
- **Correct by construction, not by shape**: the factoring only rewrites
  what the chain runs into the same statements in another place, checked
  equal, so a shape tidy doesn't know stays the `if` chain it was.

## Alternatives

- **Build the `switch` from MIR's `SwitchInt`**, its guards' blocks
  shared: refused by ADR 0629, before tidy has made the arms' chain.
- **Copy the next arm's statements into each case**, no fall-through:
  what the `if` chain did, longer, and not what React writes.
- **A labeled block per falling arm**, `fall: { .. }`: rust-js writes no
  label a person wouldn't.
- **No `// Fallthrough`**: ESLint's recommended `no-fallthrough` would
  flag each, and a reader takes a case that falls for a bug.

## Consequences

- `js::StmtKind::Switch`'s cases are `js::Case`s, each with whether it
  falls. No corpus program's or snapshot's JS but the new case's changed;
  `test/corpus/switch_fallthrough.rs` runs beside native Rust, and a
  page of the guide shows the example above.
- `Expr::negated_test` keeps where a chain of tests is written, so tidy
  finds a negated or-pattern's arm; tidy's `turned` runs after the
  `switch`, as a falling case's guard may be negated.

## What holds it

- `test/lowering-switch-fallthrough.test.ts`, "a guarded arm whose
  constants the next arm names falls through to its case": React's
  `getComponentNameFromFiber`, run against upstream's JS for each tag and
  flag; one with statements after the `match`; and one whose next arm
  names another constant, an `if` chain.
- `test/corpus/switch_fallthrough.rs`: guards that log they ran, in a
  loop of `break`s and `continue`s, against native Rust.
- Mutations in `scripts/mutations/lower/mir/hints.ts`,
  `scripts/mutations/lower/mir/tidy/switches.ts`,
  `scripts/mutations/to_oxc/statements.ts` and `scripts/mutations/to_oxc.ts`.

## Amendment: the last clause has no `break` tidy ended it with

React's scheduler's `unstable_runWithPriority` ends its `switch` with
`default: priorityLevel = NormalPriority; }`. Of a guarded case that falls
through, tidy ends each case of the `switch` with its `break;`, the one
MIR's block that ends them had, and the `default`'s too: `default: level
= 0; break; }`, a `break;` JS falls out of anyway, which upstream never
writes.

**The last clause, a `default` or the last case where there's none, has
no `break;` at its end**, whether the printer would add it or tidy left
it: `to_oxc`'s `case_body`, which already added none there (ADR 0629,
amended), drops one it's given. A `break;` in its `if`, not last, stays.

Case: N. A `break;` at the end of a `switch`'s last clause goes where the
clause's end does.

- `test/lowering-switch-ends.test.ts`, "a default after a guarded case
  needs no break", run; `test/corpus/shadow_set_in_cases.rs` beside native
  Rust, its JS without the `break;`.
- Mutation `last-clause-tidy-break-kept`.

## Amendment: a comment before an arm fallen to is before its `// Fallthrough`

React's getComponentNameFromFiber writes `// The display name for these
tags come from the user-provided type:` before `// Fallthrough` and
`case IncompleteClassComponent:`; its port writes it before the arm
`VIEW_TRANSITION_COMPONENT | INCOMPLETE_CLASS_COMPONENT | .. if
DISABLE_LEGACY_MODE => {}`, whose case the arm before falls through to.
It printed after the synthesized `// Fallthrough` and the labels, in the
case's body, as ADR 0590 places a comment before any arm. **A comment
written right before an arm whose case the one before falls through to
is before that case: before its `// Fallthrough` and its labels**, its
pattern's other constants, the earlier case's, between it and the case's
first value: such a case is a start a comment is before, as a statement
is, attached to the case itself (`to_oxc/cases.rs`, `case_starts`), on
the line of a one-line arm too. Another arm's comment is
its body's still, as the scheduler's `switch` has `case
ImmediatePriority:\n // Times out immediately`. Case N: comments, which
nothing runs. `switch_fallthrough.rs`'s reads so. Held by
`test/lowering-switch-fallthrough.test.ts`, "a guarded arm whose
constants the next arm names falls through to its case", run in JS,
`switch_fallthrough.rs` against native Rust, and the mutations
`arm-comment-case-no-start`, `arm-comment-attached-inside`,
`arm-comment-of-an-unguarded-arm-in-its-body` (`to_oxc.ts`) and
`arm-comment-before-every-case` (`to_oxc/cases.ts`).

## Amendment: a guard that falls to nothing beside one that falls to the next

React's `getOwnerStackByFiberInDev`, ported, has two guarded arms, the
second as clippy's `collapsible_match` asks:

```rust
VIEW_TRANSITION_COMPONENT if ENABLE_VIEW_TRANSITION => { info += .. }
VIEW_TRANSITION_COMPONENT | FUNCTION_COMPONENT | .. => {
    if work_in_progress.debug_owner.get().is_none() && info.is_empty() { .. }
}
FORWARD_REF if work_in_progress.debug_owner.get().is_none() && info.is_empty() => { .. }
_ => {}
```

Each is a case on its own, ADR 0636's and this one's; together they were
labeled blocks, `bb91: { if (..) { ..; break bb91; } .. }`, a label no
person writes. Four things kept them so, each now as it should be:

- **A guard's negation is where the guard is written**: `!owned` is a
  negation MIR made of `owned`, of no span of its own, so tidy found no
  guard in `tag === FORWARD_REF && !owned` and took it all for the
  pattern's test (`test_at`, which now looks inside a `!`).
- **A guard MIR tests in part in an `if` inside is one `if`**: `if (tag
  === FORWARD_REF && !owned) { if (!info) { .. } }` is the case `if
  (!owned && !info) { .. }`, as upstream writes it, where neither `if`
  has an `else`, so each way on is the same; and what the arm does once
  the guard holds, not the inner `if`, says whether it leaves
  (`guarded`).
- **A case whose failed guard goes to nothing takes nothing of the arm
  others fall to**: MIR places that arm's statements after the chain,
  which the cases that go on to it take (above); a guarded arm's that
  fails to nothing goes past the `match` instead, by the `_`'s jump in its
  list, as ADR 0636 has it, and took them, so no `switch` was made.
- **A branch a tail jump was all of is gone**: `drop_tail`, which takes a
  `break` that goes where the code goes on anyway, left `if (!on) {}
  else { .. }`, which no chain reads; it now asks `emptied_branch`, the
  one owner of an emptied branch, `if (on) { .. }`.

And **a `break;` that ends a branch of the `if` a case ends with is
gone**, where the case doesn't fall: the case's end leaves the `switch`
there anyway, by the `break;` the printer writes after it or by the
`switch`'s end, `case FORWARD_REF: if (..) { info += ..; }`, as React
writes it. So the port is upstream's `switch`, line for line, but its
last case's `break;`, which JS falls out of (ADR 0629, amended).

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each value takes
the cases Rust's arms are, its guards run in Rust's order, each once; a
`break;` removed went where the code went on.

- `default_value_exits.rs`'s `if (!below(n)) {} else { .. }` is `if
  (below(n)) { .. }`; no other corpus case's or snapshot's JS changed.
- `test/lowering-switch-fallthrough.test.ts`, "two guarded arms, one
  falling to the next, one to nothing, are one switch", run against
  upstream's JS; `test/corpus/switch_guards_apart.rs` against native Rust.
- Mutations `switch-negated-guard-unplaced`, `switch-guard-inner-if-apart`,
  `switch-failing-guard-goes-after`, `switch-case-ending-break-kept`
  (`tidy/switches.ts`) and `tail-jump-branch-left-empty`
  (`tidy/jumps/tails.ts`).

## Amendment: a fall-through the program says is said once (2026-10-26)

React's createFiberFromTypeAndProps writes `// falls through` before a
case fallen to, and its port writes it before the arm. rust-js wrote its
own `// Fallthrough` too, two comments saying one thing:

```js
    // falls through
    // Fallthrough
    case REACT_VIEW_TRANSITION_TYPE:
```

**Where a comment the program wrote before the arm says it falls through,
as ESLint's `no-fallthrough` reads one, `/falls?\s?through/i`, no
`// Fallthrough` is written beside it** (`to_oxc/cases.rs`'s
`says_fallthrough`, `fallthroughs`). Held by the corpus's
`option_constants_switch.rs` and mutation `fallthrough-said-twice`.

## Amendment: a guard whose next arm's test is in its `else` falls through (2026-10-26)

React's createFiberFromTypeAndProps falls through four guarded cases,
`legacy hidden`, `view transition`, `scope`, `tracing marker`. Its port's
arms are of the registry's symbols, which MIR tests by their `==`, and of
the third arm's case MIR wrote its guard's `if` with the fourth's test in
its `else`: `if (ENABLE_SCOPE_API) { return "scope"; } else { if
(ENABLE_TRANSITION_TRACING) { .. } }`. None of the forms `guard_alone`
read was that, so the `match` was an `if` chain with a labeled block,
`bb101:`, where upstream's is a `switch`.

**`if (guard) { .. } else { E } R`, its `if` leaving, is the guard's
`if` where `E` and then `R` are the next case's statements**: where the
guard fails it goes on to `E` and `R` either way. Held by
`test/lowering-switch-fallthrough.test.ts`, "four guarded arms of
symbols, each falling to the next, are one switch", run as JS, and
mutation `guard-else-falling-unread`.

## Amendment: a comment before the `_` arm fallen to is before its `default` (2026-10-27)

React's getTag writes `// Fall through` before `default: {`, and its port
before `_ => {`. The comment's next JS was the `default`'s first test,
past the arm's `_ => {`, which no rule let a comment pass, so it was
printed before the `switch`, and the `default` a synthetic `//
Fallthrough`. **A comment before a `_` arm whose `default` a case falls
to is before that `default`**, `// Fall through\n default: {`: the arm's
opening, `_ => {`, is passed as a closure's is, where what follows is the
`default`'s start (`to_oxc`'s comments). Held by
`test/lowering-labels.test.ts`, "guarded arms that return calls fall
through in one labeled switch", and mutation
`default-arm-comment-before-switch`.

## Amendment: a guard's `break` reaches what's after the `switch` too (2026-10-27, a wrong answer)

Case ([0262](0262-when-rust-and-js-disagree.md)): results changed; Rust's
behavior preserved.

React's completeWork writes `case IncompleteFunctionComponent: { if
(disableLegacyMode) { break; } // Fallthrough }`; its port, `A if
legacy_off => {}` before `A | B | C => { ..; return None; }`, with a
`panic!` after the `match` (cw2 of the port's gaps). `js::ends` took a
`switch` each of whose cases leaves for one that never goes on, a case
that leaves by a `break` of no label too, which goes on to what follows
the `switch`. So `blocks_one_case_reaches` (this ADR's) moved the panic
into the `default`, the one case whose `break inner` reached it, and the
guarded case's own `break` then ran what was after the `switch`, the next
arm's body. `complete(A, true)` panics in Rust; its JS called `work(1)`
and returned. Introduced by c7ae01742, this ADR's first commit, which
began moving what follows into a case.

- **A `switch` a case's `break` of no label leaves goes on** (`js::ends`).
- **What follows a block a `switch` is in, which only one case's `break
  l` reaches, is that case's** (`case_rests::rest_one_case_reaches`), where
  nothing falls out of the block and what follows leaves, or ends the
  body, given its `return;`: the arm's body after `// Fallthrough`, as
  upstream writes it, `case B: case C: work(1); return;`, the block's
  label gone. Before, the `switch` was in `bb4: { .. break bb4; .. } work(1);`.
  It and `blocks_one_case_reaches` are `mir/tidy/case_rests.rs`, out of
  `switches.rs`, which reached the file cap.

Held by the corpus's `switch_guard_does_nothing.rs`, against native Rust,
each tag with its guard holding and not, a panic after the `match` and
code; the mutations `switch-of-a-case-break-ends` (`js/stmt_walk.ts`) and
`rest-one-case-reaches-kept-after` (`mir/tidy/case_rests.ts`, beside the moved `switch-block-one-case-reaches-kept`). The new
corpus case raises `docs/readability.txt`'s `return;` share, 10.5 to 10.8
in 10,000 lines: its arms' `return None;`, as upstream's `return null;`.

## Amendment: a guarded arm that does nothing before one of its constants alone is that one's case (2026-10-27)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same tests and
the same statements run of each value.

React's completeWork writes `case IncompleteClassComponent: { if
(disableLegacyMode) { break; } .. return null; }`; its port, `I if
legacy_off => {}` then `I => { ..; return None; }` (cw2b of the port's
gaps). An arm falls to the next only where the next names a constant of
its own (above), so this `match` was no `switch`: an `if` chain in three
labeled blocks, with cw2's case as well.

**A guarded arm whose body does nothing, followed by an arm of no guard
that names its constants and no others, is one arm of them, their case**
(`hints::written_switches`): its statements are what MIR tests of them,
the guard's `if` that leaves, then the next arm's body, `case I: if
(legacyOff) { break; } ..; return;`. With cw2's fix, completeWork's
`match` is upstream's one `switch`.

Held by the corpus's `switch_guard_does_nothing.rs`, `complete_all`, each
tag and flag against native Rust; the mutation
`guard-before-its-constants-unjoined` (`mir/hints.ts`).

## Amendment: a case of an arm that returns ends with its `return` (2026-10-27)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same jump: a
case's `break` went on to the `return;` after the `switch`.

React's popToNextHostParent, `while (hydrationParentFiber) { switch
(hydrationParentFiber.tag) { case HostComponent: ..; return; case HostRoot:
..; return; default: hydrationParentFiber = hydrationParentFiber.return; }
}` (h4 of the port's gaps). Where an `Rc` is dropped on each way out, MIR
joins the arms' `return`s into one after the `match`: `case 5: ..; break;
.. default: ..; continue; } return;`.

**A case of an arm the program wrote a `return` in (`Written::switches`,
`returns`), followed by that `return;`, ends with it** in place of its
`break` (`case_rests::returns_of_cases`), and the `return;` after the
`switch` goes once nothing reaches it; then the `continue;` its `default`
ends a loop's body with, which goes on anyway, goes too
(`continues::ends_going_on`). Of a local, `while let Some(current) =
parent`, that's upstream's loop. Of a thread-local the loop reads, the
copy `const hydrationParentFiber$1 = hydrationParentFiber;` and its test,
`while (true) { .. if (.. == null) { return; } .. }`, stay: a module's
variable the loop sets is read in place only where nothing sets it in a
loop (ADR 0277), which the loop's head would need first.

Held by `test/lowering-switches.test.ts`, "a case of an arm that returns
ends with its return", run; the mutations
`case-returns-joined-after-switch`, `case-returns-joined-return-kept`
(`mir/tidy/case_rests.ts`) and `default-tail-continue-kept`
(`mir/tidy/jumps/continues.ts`).

## Amendment: an arm fallen to that's guarded and falls to nothing (2026-10-28)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same jumps: each
value runs the guards Rust runs, in order, and goes where Rust goes.

React's attemptEarlyBailoutIfNoScheduledUpdate, as its port writes it
(bw1 of the BeginWork port's gaps):

```rust
match fiber.tag.get() {
    A => { .. }
    B if on_b => { .. }
    // Fallthrough
    B | C if on_c => { ..; return update_c(fiber); }
    _ => {}
}
bailout(fiber)
```

`B if on_b` falls to the next arm, which names `B` and more. MIR tests
`B` once, so the next arm's case is `C`'s, as ADR 0905 makes it, and its
statements MIR placed after the chain, as it places what's past the
`match`. Only the cases of arms that went on to them, a guarded arm's
that falls to the next, were given them; the next arm's own case, guarded
and falling to nothing, wasn't, so the falling case's statements were no
guard's `if` and the next's, and no `switch` was made: an `if` chain,
`bailout(fiber)` returned in each branch, and in BeginWork's thirteen
arms a labeled block, `bb3: { .. break bb3; }`.

**What MIR placed after the chain, written in the arm the guarded arms
fall to last, is that arm's own case's too**, where it's guarded and
falls to nothing (`switches::switch_at`), so the `switch` is upstream's:
`case B: if (onB) { ..; break; } // Fallthrough case C: if (onC) { ..
return updateC(fiber); }`. And with it:

- **A last `_ => {}`'s `default`, MIR's copy of what's written past the
  `match` and ending in its `return`, at the end of what the chain
  takes, follows the `switch`**, as written, `switch (..) { .. } return
  bailout(fiber);`, each case's copy of that `return` its `break`, as a
  `return` after the `switch` already made it (`returns_broken`). Not
  where an `if` of an array's test is before the `switch`, nor where
  what moves declares a name.
- **A case that doesn't fall through, `if (!onC) { break; } ..;
  return ..;`, MIR's test of a guard whose failing goes past the
  `switch`, is the arm's `if (onC) { .. }`** (`guard_held`, after
  `breaks_past_switches` made `break bb2` the `switch`'s `break`), and a
  guard of `||` MIR tests in parts, `if (!legacy) { if (lanes === 0) {
  break; } }`, is `if (legacy || lanes !== 0) { .. }`. Not a case that
  falls through, where the `break` leaves and the `if`'s end would fall
  into the next case.
- **A guard of `||` tidy joined to the arm's test, `tag === OFFSCREEN &&
  (legacy || mode !== 0)`, is where its first part is written**
  (`test_at`), as one of `&&` was, so it's the guard's `if` in its case,
  React's throwException's `case OffscreenComponent: if (disableLegacyMode
  || ..) { .. }`, not an `if` chain.

The comment written before the last `_ => {}`, which has no case, is
printed before the `switch`: left as it was.

Held by `test/corpus/switch_guard_shares_next.rs`, against native Rust,
and `option_constants_switch`'s `fallen`, whose `default: return "none";`
is the `return` after its `switch` now; mutations
`fallen-to-own-case-not-absorbed`, `default-past-match-kept`,
`guard-break-kept`, `or-guard-untested` and `guard-parts-unjoined`
(`mir/tidy/switches.ts`).

## Amendment: a chain of guarded arms beside another is one `switch`

React's commitDeletionEffectsOnFiber falls through two guarded arms in a
row, `HostHoistable if supportsResources`, then `HostHoistable |
HostSingleton if supportsSingletons`, to `HostHoistable | HostSingleton |
HostComponent | HostText`, and has a second guarded arm,
`ViewTransitionComponent if enableViewTransition`, falling to `Fragment`'s.
MIR places each arm that a guard falls to past the `match`, in a block of
its own: `bb288: { bb19: { switch (tag) { .. case HostSingleton: if
(supportsSingletons) { break; } case HostComponent: case HostText: break
bb19; .. } ..singleton..; break bb288; } ..host component.. }`.
`blocks one case reaches` moved a block's rest into the one case that
reaches it only where that case's last statement was the `break`, and it
tried the outer block first, whose `switch` was still in the inner block.

- **The case's one `break inner` may end a branch of an `if` in it**, the
  guard that otherwise falls to the next case: the rest is that branch's,
  `case HostSingleton: if (supportsSingletons) { ..; break; } // Fall
  through`. Not in a loop in the case, where a `continue` of no label in
  the rest would be the loop's; not where the case names the block twice,
  which no program the corpus or the tests have makes (a guard of `||`
  isn't a `switch`), so no mutation holds that guard.
- **A block's own blocks are tried first**, so the inner one's rest is
  its case's and its `switch` is the outer one's, whose rest is then the
  next case's.

The port's function is upstream's `switch`, with no label. Case
([0262](0262-when-rust-and-js-disagree.md)): none observable; each rest
moves to the one place that reaches it.

Held by `test/corpus/guarded_arm_chains.rs`, against native Rust; mutations
`case-rest-in-guard-unplaced` and `case-rest-blocks-outer-first`
(`mir/tidy/case_rests.ts`).

## Amendment: a guard of `&&` tested in parts falls through too

React's finishConcurrentRender guards `RootSuspendedWithDelay if
!includesOnlyTransitions(lanes) && !includesOnlyRetries(lanes)`, an arm
that does nothing, before `RootSuspendedWithDelay | RootSuspendedAtTheShell`.
MIR tests the guard in parts, `if (!a) { if (!b) { break bb38; } }`, which
`guard_alone` didn't take for a guard's `if`, so the whole `match` was an
`if` chain and a label. **A guard's `if` whose branch is one `if` of no
`else`, down to the guard's end, is one test of its parts joined by `&&`**,
`if (!includesOnlyTransitions(lanes) && !includesOnlyRetries(lanes)) {
break; } // Fallthrough`, upstream's `switch`. Each part is still tested
where Rust tests it, in order, short-circuit stopping where it does.

Held by `guarded_arm_chains`' `finished`, against native Rust; mutations
`and-guard-parts-unjoined` and `and-guard-first-part-dropped`
(`mir/tidy/switches.ts`).

Of comparisons, `4 if lanes != 1 && lanes != 2 => { // .. }`, MIR tests
the parts as `if (a) { if (b) { .. } else { break bb4; } } else { break
bb4; }`, each failing part jumping the same way past the arm, which the
calls' form, `if (!a) { if (!b) { .. } }`, doesn't: it printed `bb4: {
switch .. case 4: if (lanes !== 1) { if (lanes !== 2) { } else { break
bb4; } break; } ..`, a label and an empty `then` (lb2). **`if (a) { if (b)
{ s } else { j } } else { j }`, `j` one jump alike, is `if (a && b) { s }
else { j }`**, each part a comparison of one subject with a constant, as
a `switchInt` makes them, not a presence test (`tidy/jumps/tails.rs`'s
`joined_ands`, before `branch ends`), which the rule above then takes: `if (lanes !== 1 && lanes !== 2)
{ // Commit the placeholder. break; } // Fallthrough`. `b` is tested only
where `a` holds, as before. Held by `guarded_arm_chains`' `compared`,
against native Rust; mutation `joined-ands-off` (`tidy/jumps/tails.ts`).

## Amendment: a `switch` that ends a function has its cases' `break`s

React's recursivelyInsertNewFiber (ReactFiberApplyGesture) ends with a
`match` of no value, and BeginWork's attemptEarlyBailoutIfNoScheduledUpdate
with a `match` followed by its one `return bailoutOnAlreadyFinishedWork(..)`.
Upstream ends each case with `break;`. rust-js printed MIR's way to the
function's end in each case: `return;` in every case and every `if` that
ends one, `if (flags) { b(); return; } return;`; and, where the `match`'s
last guarded arm fell to nothing and MIR ended a block with it, `bb3: {
switch (..) { .. } } return bailout(..);`, the `return` copied into every
case, which `returns_broken` didn't see past the block.

- **Of a function of no value whose body ends in a `switch`, each
  `return;` MIR made in its cases is the case's `break;`** (`breaks of an
  ending switch`, `tidy/case_rests.rs`), none where the case ends or goes
  on to a `break;` anyway: `case HostText: if (flags) { a(); } break;`.
  A `return;` the program wrote stays, as does one in a loop of the case,
  whose `break` it would be.
- **MIR's copies in the cases of the `return` that follows a `switch` it
  put in a block of its own are each the case's `break`** once the block is
  gone (`copied_returns_broken`, from `breaks past switches`), as
  `returns_broken` makes them where the `return` follows the chain: one
  `return bailoutOnAlreadyFinishedWork(current, workInProgress,
  renderLanes);` after the `switch`, as upstream writes it. A `return` the
  program wrote in an arm stays.

Each is the same jump: the `break` goes on to the function's end, or to
the `return` after the `switch`. Held by `test/corpus/switch_ends_function.rs`,
against native Rust; mutations `ending-switch-returns-kept`,
`ending-switch-written-returns-broken`, `ending-switch-breaks-kept`
(`mir/tidy/case_rests.ts`) and `copied-returns-unbroken`.

## Amendment: a `default`'s last `continue` is no longer `ends_going_on`'s (2026-10-29)

The amendment of 2026-10-27 had `ends_going_on` drop the `continue;` a
`switch`'s `default` ends a loop's body with. Since 98c8bb7b1 (#389c part 2, ADR 0629
amended), no `default` reaches `tail_continues` ending in that
`continue`: the nightly's `default-tail-continue-kept`, which skipped the `default`,
survived the corpus with its snapshots, the lowering, snapshot, JSX, port,
compiler and tidy-stage suites, and changed none of React's port's JS.
The arm goes with its mutation; popToNextHostParent's test, "a case of an
arm that returns ends with its return", still holds the `default` with
no `continue;`.


## Amendment: a string's arms fall too (rd112)

A guarded arm of strings that falls to the next, React's SimpleEventPlugin,
`"keypress" if char_code(event) == 0 => return None`, then `"keypress" |
"keydown" => ..`, printed `bb22: { if (name === "keypress") { .. } else if
(name !== "keydown") { .. } ctor = 1; }`, or, in a port module of thirty
arms, `bb175:`. Whether an arm falls to the next, and whether two arms may
match one value, ask the value each constant pattern is
(`body_queries::pattern_value`), which told numbers, `char`s and the
registry's symbols, not strings: so the guarded arm wasn't known to fall,
nor to fall to nothing, and the `match` had no `switch`. A string's
pattern's value is now its text (`PatternValue::Text`), so the `match` is
the `switch` upstream writes:

```js
switch (name) {
  case "keypress":
    // Remove unwanted keypresses.
    if (code(n) === 0) {
      return -1;
    }
  // Fallthrough
  case "keydown":
    ctor = 1;
    break;
  ..
}
```

and in a loop the `if` chain Rust tries (#389b). Two string patterns of
other texts never match one value, as `may_match_alike` now tells.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same tests in
the same order.

Held by `test/corpus/switch_text_guard_falls.rs` against native Rust and
the mutation `text-pattern-untold` (`body_queries/constants.ts`).

## Amendment: a second fall-through, and a guard of reads joined by `||` (rd118)

React DOM's SimpleEventPlugin has two guarded arms that fall to the next,
`"keypress" if .. => return` and `"click" if .. => return`, and an arm of
the animation events, `name if ANIMATION_END.with(..) ||
ANIMATION_START.with(..) => ..`. Its `match` was `bb180: { .. }`: MIR
tests the arms after the first fall-through in that guard's `else`, so the
second's fallen-to statements, `ctor = 5; break bb33;`, follow its `if`
inside that branch, where only a `match`'s list's end took them
(`switch_at`); and it tests a guard of `||` in parts, `if (A !== name) {
if (B !== name) { .. } } ctor = 6;`, which no negated test read.

Now, in a branch of the chain, what follows a guarded arm that falls,
written in the arm it falls to, is each case's that goes on to it, the
falling one's and its own, as at the list's end; and nested `if`s of no
`else` are tried as one test, `A !== name && B !== name`, where their
parts are the arm's constants, each of an arm a guard compares with reads
(ADR 0629, amended), `case ANIMATION_END: case ANIMATION_START:`. Upstream's
`switch`, both guarded cases falling through.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same tests in the
same order: each guard runs once its constant matched, and the reads of
`||` in order until one matches, as a `switch` evaluates stacked cases.

Held by `test/corpus/switch_guards_fall_twice.rs` (a thread-local changed
to a value an earlier arm names) against native Rust and the mutations
`second-fall-unabsorbed`, `or-guard-parts-untried` (`mir/tidy/switches.ts`)
and `or-guard-of-reads-no-case` (`mir/hints.ts`).

## Amendment: an arm's block's comment, and a `default` left of nothing (rd119)

React DOM's SelectEventPlugin's `match` of blocks, `"focusin" => {
construct(..); }`, `"selectionchange" if SKIP.with(..) => {}`, then
`"selectionchange" | "keydown" | "keyup" => { .. }` and `_ => {}`, was its
`switch` with each comment written before an arm of a block moved above
the `switch`, and an empty `default:`. The comment's code before its
statement, `"focusin" => {`, opened nothing a comment is read past; now an
arm's opening does, and a method call's receiver, `DOWN.` of
`DOWN.set(true)`, whose span starts at `set`, so the comment is before the
case's first statement, as one before an arm of an expression is (ADR
0630). And a last `_ => {}` of a `switch` that ends a function, MIR's
`return;`, which `breaks_of_ending_switch` leaves empty, is left out, as
`breaks_past_switches` leaves out one that does nothing.

Case ([0262](0262-when-rust-and-js-disagree.md)): N.

Held by `test/corpus/switch_guard_skips_its_case.rs` against native Rust
and the mutations `arm-opening-not-an-opening` (`to_oxc.ts`),
`receiver-not-only` (`to_oxc/openings.ts`) and `emptied-default-kept`
(`mir/tidy/case_rests.ts`).
