# 0636. A guarded arm of a `match` of constants is a case

Status: Accepted. Amends [0629](0629-a-match-of-constants-is-a-switch.md),
whose `switch` had no arm with a guard. Amended by
[0905](0905-a-guarded-arm-falls-to-the-next.md): a guarded arm whose
next arm names each of its constants falls through to that arm's case;
and an or-pattern of a guard, which MIR tests by each alternative, is one
case. Amended below: a guarded arm just before a last `_` that does
something falls through to the `default`; and no `default` holds what
follows the `match` past a failed guard (rd184).

Case ([0262](0262-when-rust-and-js-disagree.md)): N, nothing the program
can observe: the case runs the guard where the arm did, after its
constant matched, and where it fails goes on to what Rust's next try goes
on to, nothing.

## Context

React's `getComponentNameFromType` writes a case whose statements test a
flag:

```js
case REACT_TRACING_MARKER_TYPE:
  if (enableTransitionTracing) {
    return 'TracingMarker';
  }
}
```

Its port wrote it as Rust's arm of an `if`, which clippy's
`collapsible_match` asks to be a guard, `Some(REACT_TRACING_MARKER_TYPE)
if ENABLE_TRANSITION_TRACING => return ..`. A `match` with a guard wasn't
a `switch` (ADR 0629), so the whole `match` became an `if` chain of every
arm, the six before it too.

## Decision

**A guarded arm is a case whose statements test the guard, `case C: if
(guard) { .. } break;`, where what Rust tries when the guard fails does
nothing: each later arm whose pattern could match a value of the arm's is
a last `_ => {}`, or there's none.** Otherwise a failed guard goes on to an
arm that does something, which a case can't, and the `match` stays an `if`
chain.

```rust
match kind {
    FRAGMENT => return Some("Fragment"),
    PROFILER => return Some("Profiler"),
    TRACING_MARKER if tracing => return Some("TracingMarker"),
    _ => {}
}
```
```js
switch (kind) {
  case FRAGMENT:
    return "Fragment";
  case PROFILER:
    return "Profiler";
  case TRACING_MARKER:
    if (tracing) {
      return "TracingMarker";
    }
    break;
}
```

- **One place decides it**: `hints::written_switches`, which decides each
  arm of a `switch`, asks `fails_to_nothing` of a guarded one. Whether a
  later arm could match is `may_match_alike`: two variants are apart where
  they're different variants, two constants where the value each pattern
  is, `body_queries::pattern_value`, the one answer of which value a
  constant pattern is (ADR 0590's, now a function of its own), differs; a
  string's or a float's, which that doesn't tell, could. A later arm of
  the same constant, `C if on => .., C => ..`, could, and is an `if` chain.
- **Its case is what MIR made of it**: tidy (`tidy/switches.rs`) reads the
  arm's `if` as it reads an unguarded arm's, its pattern's test then what
  it does; of a test the guard's `&&` joins, `n === C && on`, the guard is
  its own `if` in the case (`guarded`), and an `&&` the program didn't
  write is where its first test is (`test_at`). Tidy decides nothing of
  the guard: a case's failed guard reaches what follows the `switch`, as
  the `if` chain's reached what follows the `match`, as `_ => {}` does
  nothing.
- **Not a `switch`, still**: an or-pattern of a guard, `B | C if a`, which
  MIR tests by each alternative, a guard of its own each, is an `if`
  chain as ADR 0629 has one whose chain tidy shaped another way.

## Why

- **It's React's `switch`**, and what a person writes of a case that does
  something only where a flag's on.
- **One rule**: the guard is tested where Rust tests it, after its
  pattern, and its failure reaches what Rust's does. Where it can't, the
  `if` chain says what Rust does, as every `match` did before ADR 0629.
- **Clippy's Rust is what's written**: the `if` in an arm that
  `collapsible_match` asks to be a guard is the same `switch` either way.

## Alternatives

- **A guarded arm's case falling through to the next case** that could
  take it, `case C: if (on) { .. } // falls through`: JS's fall-through
  runs the next case's statements, not its test, and the cases between
  would run too. Rejected.
- **`switch (true) { case n === C && on: .. }`**: a trick, not what React
  writes (ADR 0629).
- **The `if` chain of every guarded `match`**: what React's port read,
  seven tests where React writes a `switch`.

## Consequences

- No corpus program, snapshot or page of the guide but the one example
  added changes: none had a guard after which only `_ => {}` came.
- `SwitchArm` holds where its guard is written.

## What holds it

- `test/lowering-switches.test.ts`, "a guarded arm of a match of
  constants is a case that tests its guard": React's shape, a guard nested
  and joined, a guard in a loop, a guard followed by `_ => 0` and by an arm
  of the same constant, each run for the guard true and false.
- Mutations in `scripts/mutations/lower/mir/hints.ts` and
  `scripts/mutations/lower/mir/tidy/switches.ts`.

## Amendment: a guarded arm just before a last `_` that does something falls through to the `default`

React's `describeFiber` is a `switch` that's its function's answer, its
last case falling through to its `default`:

```js
case ViewTransitionComponent:
  if (enableViewTransition) {
    return describeBuiltInComponentFrame('ViewTransition');
  }
// Fallthrough
default:
  return '';
```

Its port writes the `match` that's its value, `VIEW_TRANSITION_COMPONENT
if ENABLE_VIEW_TRANSITION => .., _ => String::new()`. A failed guard went
on to an arm that does something, which this ADR's case couldn't, so the
whole `match` was an `if` chain, `if (tag === HostComponent) { return ..;
} .. return "";`.

**A guarded arm just before a last `_`, or a last name alone, that does
something falls through to the `default`**: its case is the guard's `if`,
`// Fallthrough` before `default:`, as ADR 0905's case falls to the next
arm's. What Rust tries where the guard fails is that `_`, which takes
every value, and JS's fall-through runs the `default`'s statements, the
next clause's. Of a guarded arm with an arm between it and the `_`, which
JS can't fall past, the `match` stays an `if` chain; of a `_ => {}`, the
case is as above, `if (g) { .. }` then nothing.

- **Which arm falls is the hint's** still, `hints::written_switches`:
  `falls` of a guarded arm whose next arm is the last, a catch-all with no
  guard, where it doesn't fail to nothing (`fails_to_nothing`). The `_`'s
  case is the `default`, of no constant.
- **The case is what MIR made**: MIR tests `tag === C && g`, and where
  that fails goes on in the chain's list to the `_`'s statements; tidy
  (`tidy/switches.rs`, `fallen`) takes the guard's `if` alone, which
  leaves where it holds, as the case that falls to the `default`, its
  next clause.
- **Printing**: `// Fallthrough` before `default:`, at its first
  statement's start, as before a case's first value
  (`to_oxc/cases.rs`, `default_fallen_to`); a comment written before the
  `_` arm is before it.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each value runs the
guard Rust runs, once, after its constant matched, and then what the arm
Rust takes does.

- No corpus case's or snapshot's JS but the new ones changed.
- `test/lowering-switch-fallthrough.test.ts`, "a guarded arm before a
  last _ that does something falls through to the default", run;
  `test/corpus/switch_guard_to_default.rs` against native Rust, of a value
  `match` and of a statement one whose last arm is a name.
- Mutations `switch-guard-to-default-no-fall`,
  `switch-default-after-fall-unvalued` (`hints.ts`),
  `switch-default-fallen-refused` (`tidy/switches.ts`) and
  `default-fallthrough-unsaid` (`to_oxc/cases.ts`).

## Amendment: no `default` of what follows the `match` past a failed guard (rd184, 2026-10-29)

A wrong answer. React DOM's clearContainerSparingly, as its port writes
it:

```rust
match node.node_name().as_str() {
    "HTML" | "HEAD" | "BODY" => { ..; continue; }
    "SCRIPT" | "STYLE" => { continue; }
    "LINK" if is_stylesheet(node) => { continue; }
    _ => {}
}
container.remove_child(node);
```

A `<link>` that isn't a stylesheet fails the guard, goes on to the `_`,
which does nothing, and then to `remove_child`. MIR has `remove_child`
where the `_`'s arm is, in the branch of the tests every arm but the
first two goes on to, and `chain` gave all of it to the last arm, the
`default`: `case 'LINK': if (..) { continue; } break; default:
container.removeChild(node);`. The case's `break` goes past the
`switch`, skipping the removal.

**Where a guarded arm's case goes on past the `match` where its guard
fails, and the `default` holds what MIR has of what's written past the
`match`, there's no `switch`**: its arms stay the `if` chain they are,
`if (name === "LINK") { if (..) { continue; } } container.removeChild(node);`.
A `default` that holds none of that, or whose `return` the `switch` is
followed by once it's moved after it (ADR 0905, amended), is a `switch`
as before. Upstream's own form, the `switch` with `removeChild` after it
and each arm's `continue` (kj-switch), is a readability change after this
one; this is the correct form.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior: the arm after a failed guard, and then what follows the
`match`.

Held by `test/corpus/guarded_arm_falls_to_tail.rs`, against native Rust
(of strings, a `switch` of integers whose tail stays after it, and arms
after the guarded one that do something); mutations
`guard-failing-past-default-kept` and `guard-failing-past-any-default`
(`mir/tidy/switches.ts`).
