# 0905. A guarded arm whose constants the next arm names falls through to its case

Status: Accepted. Amends [0636](0636-a-guarded-arm-is-a-case.md), whose
guarded arm was a case only where its failed guard went on to nothing,
and [0629](0629-a-match-of-constants-is-a-switch.md), whose cases each
ended where they went on. Amended below: the last clause has no `break`
tidy ended it with.

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
