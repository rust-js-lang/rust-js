# 0629. A `match` of constants is a `switch`

Status: Accepted. Amends [ADR 0014](0014-match-lowering.md), which made
every `match` an `if` chain. Amended by
[0731](0731-a-cases-break-past-the-switch-is-its-own.md): a case's `break`
past the `switch` is its own; by
[0634](0634-a-registered-symbol-is-a-const.md): a `Some` of constants,
where it's the value itself, is those constants, and the registry's
symbol, `js::SymbolFor`, is one; by
[0636](0636-a-guarded-arm-is-a-case.md): a guarded arm is a case that
tests its guard, where a failed guard goes on to nothing; by
[0668](0668-a-match-of-what-a-value-is-is-a-switch-of-its-typeof.md): a
`match` of an untagged enum's kinds is a `switch (typeof value)`, and an
arm's test is found wherever MIR tests it, its cases written as the arms
are. By [0905](0905-a-guarded-arm-falls-to-the-next.md): a guarded
case falls through to the next where the next arm names its constants.
By [0958](0958-a-chain-of-strict-equals-tests-is-a-switch.md): an
`if` chain of `js::strict_equals` tests of one subject is a `switch` too.
Amended below: a last arm of a name, `current => current`, is the
`default`, as a last `_` is.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, nothing the program
can observe: a `switch` compares what it's given with each `case` by
`===`, as each arm's test did, and runs what the matching arm did.

## Context

React writes a choice among constants as a `switch`, everywhere:
`switch (workInProgress.tag) { case FunctionComponent: .. }`, `switch
(formatString.charCodeAt(++i)) { case 79: case 99: .. }`. rust-js wrote
every `match` as an `if` chain, `if (tag === FunctionComponent) { .. } if
(tag === ClassComponent) { .. }`, and where MIR held what's matched, a
temporary before it, `const charCodeAt = s.charCodeAt(i); if (79 ===
charCodeAt || ..)`. ADR 0014 refused `switch` because a `break` in a JS
`switch` leaves the `switch`, where Rust's leaves the loop. A label
answers that, as React's own code does: `outer: while (true) { switch
(..) { .. break outer; } }`. The user decided (2026-10-23) a `match` of
constants is a `switch`.

## Decision

**A `match` the program wrote of three cases or more, each constant of
its patterns a case and a last `_` one, each arm a constant or an
or-pattern of them, with no guard and no binding, and at most a last
`_`, is a `switch`; the `_` is its `default`.** A constant is a literal, of
a number, a `char` or a string; a `const`, by its name (ADR 0590); or an
enum's variant of no field that JS tests as a value it's equal to,
`"Function"` (ADR 0013), or an `Ordering`'s number. Which
`match` is one is answered once, from THIR, by `hints::written_switches`,
and carried to tidy as `Written::switches`: each arm's span, its
pattern's, and its constants.

```rust
match c {
    'a' | 'e' => n += 1,
    '.' => break,
    '#' => continue,
    _ => {}
}
```
```js
loop: for (const c of s) {
  switch (c) {
    case "a":
    case "e":
      n = (n + 1) >>> 0;
      break;
    case ".":
      break loop;
    case "#":
      continue;
  }
  ..
}
```

- **Its shape.** An or-pattern's constants are stacked `case`s. Each case
  ends in `break;` where its statements would go on, and is a block, `case
  1: { .. }`, where it declares a name, which the cases' one scope would
  share: the printer writes both, as a case's statements end it as an
  `if`'s branch's do (`js::StmtKind::Switch`). A `default` that does
  nothing, `_ => {}`, is left out. A last arm of a variant, Rust having
  proved the rest, is a `case` of its own, not a `default`: `case "Host":`.
  A comment before an arm is before its case's statements.
- **A `break` of a loop in a case names the loop**: its label, the
  program's where it wrote one, `'rows`, else `loop` (ADR 0015's), apart
  from any label around it, `loop$1`. A `continue` stays: a `switch`
  doesn't take one.
- **What's matched is read once, where it's written**: a temporary MIR
  made of it that only the tests read is its value, `switch
  (s.charCodeAt(i))`; a variable the program named keeps its name,
  `switch (code)`.
- **A value of the arms is the program's name's**, `let label; switch (n)
  { case 0: label = 10; break; .. }`, a temporary MIR assigned in each
  case then copied into it too: tidy doesn't fold a `switch`'s arms into a
  `?:` (`branches::switched`).
- **Its chain, settled**: tidy writes the `switch` last
  (`tidy/switches.rs`), from the `if` chain its other passes leave, each
  arm's test where its pattern is written, in the order written; an arm
  MIR tests negated, `if (c !== ".") { .. }`, as it does the arm before a
  last `_`, is a case too, and a case alone that goes on, what follows the
  `switch` that's written in it, MIR's `break` of the loop.

**Not a `switch`:**

- **A `match` of two cases**: `if (n === 0) { .. } else { .. }`, or its
  `?:`, as JS writes two ways, and as SonarJS's `no-small-switch` asks.
  Cases, not arms, decided with the user: React's `normalizeConsoleFormat`
  has one arm of seven constants and a `_`, a `switch` of seven cases. A
  `match` of a `bool`, which has two values, is one; its arms aren't
  constants a `switch` compares, as JS tests a `bool` by itself, `if (b)`.
- **One whose value is read in place**, an argument, an operand, a
  pattern's, `f(match n { .. })`, `let (a, b) = match ..`: JS has no
  `switch` expression, and a temporary to hold it reads worse than the
  `?:` chain, `n === 1 ? "one" : n === 2 ? "two" : "many"`.
- **Any other `match`**: a guard, a range, a binding (`other => ..`), a
  pattern of fields, or one a macro wrote, `matches!`: an `if` chain, as
  ADR 0014 has it. So is one whose chain tidy shaped another way than its
  arms, its tests out of the order written, which runs the same.
- **A table read** (ADR 0233): `VARIANTS[kind]`, which says more.

## Why

- **It's what a person writes**, React above all: one `case` per
  constant, the value read once, a label where a `break` leaves a loop.
- **One rule a Rust developer can guess**: a `match` of constants is a
  `switch`, but of two cases, the `if` a person writes of two ways, and of
  a value read in place, which JS's `switch` can't give.
- **`===` is what each arm tested**: a float pattern is never `NaN`, and
  `0.0` matches `-0` either way.

## Alternatives

- **Every `match` of constants a `switch`, of two cases too**: `switch (b)
  { case true: .. case false: .. }` of a `bool`'s, `switch (n) { case 0:
  return "zero"; default: return "other"; }`, which no JS developer
  writes; and a `let x;` and a `break` in each case for what was `const x
  = n === 0 ? a : b`.
- **A `switch` of a value read in place**, through a temporary: `let
  value; switch (n) { .. } f(value)`, a name and lines the `?:` hasn't.
- **`switch (true) { case n > 5: .. }`** of guards and ranges: a trick,
  not what React writes.
- **Building the `switch` from MIR's `SwitchInt`**: before tidy has made
  the arms' chain, its returns and its comments, each of tidy's passes
  would have to know a `switch`.

## Consequences

- Twelve corpus programs, ten snapshots and two of the guide's pages
  write their `match`es of constants as `switch`es. In
  `docs/readability.txt` the JS is 56 lines longer, 87,390 to 87,446, a
  `case` a line where a test was one; names given a `$1` shrink, 7,461 to
  7,457, a temporary of what's matched read in place, `switch (note(2,
  x))`; no shape's share grows.
- Every statement walker knows a `switch`: its cases are lists of
  statements, as an `if`'s branches are, and a `break` of no label in one
  is the `switch`'s, so a query of a loop's `break`s doesn't look for
  one there.
- A `match` whose chain tidy shapes otherwise stays the `if` chain it
  was, which runs the same; the corpus has none but the tables of ADR
  0233.

## What holds it

- `test/lowering-matches.test.ts`, "a match of constants is a switch",
  and the expectations of a `match` of constants in
  `test/lowering-branches.test.ts` and `test/lowering-matches.test.ts`.
- Mutations in `scripts/mutations/lower/mir/hints.ts`,
  `scripts/mutations/lower/mir/tidy/switches.ts` and
  `scripts/mutations/to_oxc/statements.ts`.

## Amendment: a `break` only where a case goes on to another

**A case ends in `break;` only where it goes on and a case follows it**:
the last of a `switch`'s, which JS falls out of, has none, as React's
`describeElementType` ends `case REACT_VIEW_TRANSITION_TYPE: if
(enableViewTransition) { return 'ViewTransition'; } }`; and one that
never goes on has none, of a `try` too, each of whose arms leaves, `try
{ return ..; } catch { return ''; }` (`js::ends`, which knew no `try`).
Case N: a `break` there does nothing.

- Every `switch`'s last case loses its `break;`: four corpus cases, two
  snapshots, a guide page and the tests that held one. `catch_unwind_resumed`,
  whose `else` ended in a `try` that rethrows, now reads its guard first,
  `if (result.TAG === "Err") { .. }`, then the drops, as `js::ends` says
  that `try` never goes on. `docs/readability.txt` counts seven lines
  fewer, which moves the renames' share up a tenth: no more of them.
- `test/lowering-switch-ends.test.ts`, "a case needs no break where it
  leaves or it's the last"; mutations `last-case-broken` and
  `tried-case-broken`.

## Amendment: a last `_`'s statements are its `default`, and an `Option`'s text is the value's `switch`

React's `resolveThenable` (ReactChildren.js), ported, read

```js
switch (thenable.status) {          // upstream: the same, and
  case "fulfilled": return ..;      //   default: {
  case "rejected": throw ..;        //     if (typeof thenable.status === ..) ..
}                                   //     switch (thenable.status) { .. }
if (typeof thenable.status ..) ..   //   }
const status = thenable.status;
if (status != null) {
  switch (status) { ..; default: break; }
}
```

MIR tests the arms before a last `_ => { .. }` as an `if` and its `else`,
and puts the `_`'s statements after that chain, which the `switch` left
after it; and of `match s.as_deref() { Some("a") => .., _ => {} }` it
tests `None` first, `if (status == null) { break bb; }` or `if (status !=
null) { .. }`, around the `switch`, its `_` a `default` of that `break`.

**A last `_`'s statements are its `default`**: of what follows the chain,
what's written in the arm or MIR's, up to the first statement written past
it, which stays after the `switch`, where nothing there reads what the
`default` then declares. **A `default` of a `break` alone is none**; and
**a test of `None` before a `switch` of the value**, `if (x == null) {
break l; } switch (x)` ending block `l`, or `if (x != null) { switch (x)
}`, **goes**, where each case is a literal and the `default` does nothing:
`switch` takes what no case is, `null` and `undefined` too, as Rust's `_`
does. The temporary its test read, the `switch`'s alone, is then read in
place, `switch (thenable.status)`. Case N: each moves a statement only
one path reaches, or drops a test whose branches do the same.

- `test/lowering-switches.test.ts`, "React's resolveThenable is upstream's
  switch, its last arm its default"; `test/corpus/switch_option_text.rs`,
  beside native Rust. No other corpus case's or snapshot's JS changed.
- Mutations `default-statements-after-switch`,
  `none-guard-kept-before-switch`, `none-guard-around-switch-kept`,
  `default-break-kept`, `guarded-switch-temporary-kept`.

## Amendment: tidy alone leaves out a `default` that does nothing (#268)

The printer also filtered out an empty `default`, which tidy's
`switches` stage had already removed (`does_nothing`). It was a second
authority, and its mutation survived. The printer now prints the
`default` it's given. Tidy's rule is held by
`switch-default-doing-nothing-kept`, which is caught.

## Amendment: a last arm of a name is the `default`

React's scheduler, ported, picks the priority `unstable_next` runs at:

```rust
let priority_level = match CURRENT_PRIORITY_LEVEL.get() {
    // Shift down to normal priority
    IMMEDIATE_PRIORITY | USER_BLOCKING_PRIORITY | NORMAL_PRIORITY => NORMAL_PRIORITY,
    // Anything lower than normal priority should remain at the current level.
    current => current,
};
```

Upstream's `Scheduler.js` is a `switch`, each comment in its case:

```js
switch (currentPriorityLevel) {
  case ImmediatePriority:
  case UserBlockingPriority:
  case NormalPriority:
    // Shift down to normal priority
    priorityLevel = NormalPriority;
    break;
  default:
    // Anything lower than normal priority should remain at the current level.
    priorityLevel = currentPriorityLevel;
    break;
}
```

A binding arm was "any other `match`", an `if` chain: the port's JS was an
`if`/`else`, then, since ADR 0658, `const priorityLevel = current ===
ImmediatePriority || .. ? // Shift down to normal priority NormalPriority :
// Anything lower .. current;`, its comments inside the `?:`.

**A last arm whose pattern is a name alone, with no guard, `current =>
..`, is the `default`, as a last `_` is**, the name the value matched:
what `_` matches, it matches, and binds. One of a `@` pattern, `n @ 1..=3`,
or a binding before the last arm, still makes the `match` an `if` chain.
So `unstable_next` is upstream's `switch`:

```js
const current = currentPriorityLevel;
let priorityLevel;
switch (current) {
  case ImmediatePriority:
  case UserBlockingPriority:
  case NormalPriority:
    // Shift down to normal priority
    priorityLevel = NormalPriority;
    break;
  default:
    // Anything lower than normal priority should remain at the current level.
    priorityLevel = current;
}
```

Case N, as before: the `default` runs where no case's `===` holds, as the
last arm did. Decided while the user was away, as one rule a Rust
developer can guess: `_` and a name are the two ways Rust writes a
`match`'s last, catch-all arm, and a `switch` of constants is one form
whichever the program writes; the scope the user decided, three cases or
more and not a value read in place, is unchanged. A `match` of two
constants and a name, `0 => .., 1 => .., n => ..`, is three cases, a
`switch`, as one with a last `_` was.

- `user_from_str.rs`'s `match s.trim().to_lowercase().as_str() { "admin"
  => .., "member" => .., other => .. }` and the guide's `Score` (`0`, `1`,
  `n`) are `switch`es; no other corpus case or snapshot changed.
- `test/port-scheduler.test.ts`'s snapshot holds `unstable_next`.
- Mutation `switch-last-name-an-if-chain` (`hints.ts`).


## Amendment: an `Option`'s constants are the value's test, and a last clause's jump the switch's (2026-10-26)

React's createFiberFromTypeAndProps, ported, matches an `Option` of
constants inside a labeled block its arms leave, `'get_tag: { match sub {
Some(7) => { ..; break 'get_tag; } Some(8) => { ..; break 'get_tag; } _ =>
{} } tag = "throw"; }`. MIR tests it twice, its `Some` and then its
payload, and its `None` and its payload's other values both go to the
`_` arm: a block two edges reach, so a labeled block, `bb1: { if (sub !=
null) { switch (sub) { .. default: break bb1; } return tag; } } tag =
"throw";`. A JS label is never what a person writes where the program
wrote none (the user's rule).

- **A test that an `Option` of a number, a `char` or text is `Some` whose
  `Some` goes on to a test of its payload alone, and whose `None` goes
  where that test's `otherwise` does, is that test, of the value itself**
  (`patterns.rs`'s `mir_payload_tests`, a body's `LocalFacts`): its graph
  goes on where the payload's test does (`cfg::Graph::of`), and its
  terminator is that test's, `sub === 7`, which no `None` passes, as no
  `null` or `undefined` is `7`. The `_` arm is reached once, as a match
  of a number's is: `switch (sub) { case 7: ..; break; case 8: ..; break;
  default: tag = "throw"; }`. Not of a `bool`, which JS tests by its
  truth, `!b`, which `None` passes; nor where `None` has its own arm.
- **A `switch`'s last clause that ends with the jump the `switch` is
  followed by ends without it**, as its end goes on to it
  (`breaks_past_switches`): `default: tag = "throw";` before `return
  tag;`, where MIR copied the tail into it. Only a jump, which ends what
  runs either way; a statement that isn't one would run twice, which no
  body MIR makes has, so no test reaches that check.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same tests and
the same statements run of each value.

Held by the corpus's `option_constants_switch.rs`, against native Rust:
a labeled block left from an `Option`'s arms, a `char`'s through a
reference, one whose `None` has its own arm, a `bool`'s; and mutations
`payload-test-unfolded`, `payload-test-of-none-elsewhere`,
`payload-test-of-a-bool` (`mir/patterns.rs`), `facts-held-arms-unjoined`
(`mir/local_facts.rs`, its find moved) and `last-clause-tail-kept`
(`mir/tidy/switches.rs`).

## Amended (2026-10-28): no last clause's jump is taken off

The tail MIR copied into a `switch`'s last clause, `default: tag =
"throw"; return tag;` before `return tag;`, isn't there now: the passes
before `breaks_past_switches` leave the `switch`'s join where it's
written, so its `default` is `tag = "throw";` already. No corpus case,
lowering or port test, nor `option_constants_switch.rs` or a late-set
`let tag; match .. { .. }`, reaches the check, and its mutation,
`last-clause-tail-kept`, survived every run: the check is gone with it
(ADR 0093).
