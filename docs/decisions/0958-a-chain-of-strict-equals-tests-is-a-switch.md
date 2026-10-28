# 0958. A chain of `js::strict_equals` tests of one subject is a `switch`

Status: Accepted (the user's decision, 2026-10-27). Amends
[0629](0629-a-match-of-constants-is-a-switch.md), a `match` of constants
as a `switch`, to an `if` chain of `===` tests the program wrote.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. A `switch` reads its
subject once and compares it with each case's value in order, `===`, until
one matches, and runs that case's statements: what the chain did, where
reading the subject again gives the same value and a value does nothing.

## Context

React's ReactTestSelectors tests a selector's tag, a JS value, with
`switch (selector.$$typeof) { case COMPONENT_TYPE: .. }`. Its tags are
`Symbol.for(..)`s, or numbers where symbols aren't, values no Rust `match`
of constants can name, so its port asks `js::strict_equals(selector.typeof,
COMPONENT_TYPE.get())`, an `if`/`else if` chain, and the JS was that chain:
`matchSelector`'s and `selectorToString`'s `if (selector.$$typeof ===
COMPONENT_TYPE) {..} else if (..)`, not upstream's `switch`.

## Decision

**An `if`/`else if` chain of three ways or more, each test a
`js::strict_equals(s, v)` the program wrote, of one subject `s` that reads
the same each time (`reads_same`: no call, no getter) and values `v` that
do nothing, is `switch (s) { case v: .. default: .. }`**, its last `else`
the `default` (`tidy/strict_switches.rs`). The hints say where each
`js::strict_equals` call is (`Written::strict_tests`), as they say where
each `&&` is.

MIR's structure makes guards of branches that never go on: `if (s === A)
{ return a; } rest`. Such a guard is a link of the chain where the program
wrote its `if` with an `else` (`Written::elses`) and `rest`, to the list's
end, is that `else`; `if (s !== C) { throw .. } rest` is `else if s === C
{ rest } else { panic!() }`'s. A chain of values whose last two ways
`returned_values` made a returned `?:` of, `return s === B ? b : c`, ends
with `case B: return b; default: return c;`. The pass runs before
`returned values`, which would make one of the last.

A chain stays as written where a `switch` would change what it does or
how it reads:

- its tests mix subjects, `s === A` then `t === B`: none of it is a
  `switch`, not even its tail's tests of one;
- a branch `break`s a loop the chain is in, which a `switch` would take;
- two ways only, an `if` and an `else if`, which a person writes as such;
- a guard the program wrote in the last `else`, `else { if !eq(s, c) {
  return 3; } .. }`, is the `default`'s statements, and statements after a
  chain in an `else` are after the `switch`.

## What holds it

- `test/lowering-strict-switches.test.ts`: "React's matchSelector's
  chain of strict_equals tests is a switch" and "a chain of strict_equals
  tests a switch would change stays", each compiled and run.
- Mutations (`scripts/mutations/lower/mir/tidy/strict_switches.ts`):
  `strict-switch-of-no-strict-equals`,
  `strict-switch-of-a-subject-read-again`,
  `strict-switch-of-a-value-that-does-something`,
  `strict-switch-taking-a-loops-break`,
  `strict-switch-of-mixed-subjects-part`, `strict-switch-of-two-ways`,
  `strict-switch-guard-not-written-with-else`,
  `strict-switch-dropping-what-follows-an-else`,
  `strict-switch-of-a-returned-conditional`,
  `strict-switch-returned-value-where-written`.
- The guide's `js::strict_equals` example, `docs/guide/interop/types.md`.

## Consequences

A chain of values assigned, `let k = if eq(s, a) { 1 } else if .. `, is
`?:`s still, an expression's, not a `switch`.

## Amendment: a guard's `!==` isn't read as a negated test (2026-10-28)

`if (s !== C) { throw .. } rest` was read as `s === C`'s test negated,
so React's `selectorToString` was a `switch`. The chain now reaches this
pass as the program wrote it, `else if s === C { .. } else { panic!() }`,
before it's a guard: `written chains` puts each `if` back with its `else`.
Under the mutation `strict-switch-of-negated-guards`, which took out the
reading of `!==`, the JS was unchanged everywhere: the corpus with its
snapshots, and the lowering, snapshot, JSX, port, compiler and tidy-stage
suites. A program's own `else if !js::strict_equals(s, c) { 3 } else { 4 }`
was never made a case either way, as its `if` isn't written with its
branches turned. So a chain's test is `s === value` alone
(`strict_switches::tested`). The reading of a negated test went, and so
did its mutations, `strict-switch-of-negated-guards` and
`strict-switch-negated-test-as-written`.
