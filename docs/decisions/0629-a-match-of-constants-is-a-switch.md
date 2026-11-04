# 0629. A `match` of constants is a `switch`

Status: Accepted. Amends [ADR 0014](0014-match-lowering.md), which made
every `match` an `if` chain. Amended below, the user's decision of #389: no
label the program didn't write, superseding `loop:`. Amended by
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
`default`, as a last `_` is; and, the user's decision of 2026-11-03, the
last clause that goes on ends with `break;` too, superseding "a `break`
only where a case goes on to another".

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

(Superseded below, 2026-11-03: the last clause that goes on ends with
`break;` too.)

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

## Amendment: what's written after the `match` stays after the `switch` (2026-10-28, rd49)

ReactDOMUnknownPropertyHook writes `switch (name) { case 'inert': {
break; } default: { return true; } } console.error(..); return true;`.
Its port's `match name { "checked" | "inert" => {} _ => { return true; } }
error(..); return true;` printed `case "checked": case "inert":
console.error(..); return true; default: return true;`: MIR tests the
arm that does nothing negated, `if (t) { return true; } else { .. }`, its
`else` what follows the `match`, which the case took as its own.

**Of an arm's test whose branch leaves, `if (t) { exit } else { own; rest
}`, what's written after the whole `match`, `rest`, is after the `if`
before the `switch` is made** (`switches.rs`'s `written_after`): the case
is `own`, a `break` where that's nothing, and `rest` follows the
`switch`. Case ([0262](0262-when-rust-and-js-disagree.md)) N: `exit`
leaves, so `rest` runs only after `own`, either way.

Held by `test/corpus/arm_returns_as_written.rs`, against native Rust;
mutation `switch-takes-what-follows` (`tidy/switches.ts`).

## Amendment: no label the program didn't write (#389)

The user decided, of #389, that a JS label never appears where the Rust
wrote none, which supersedes this ADR's `loop:` above: "a `break` of a loop
in a case names the loop" now names it only where the program labeled it.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each form runs the
same jumps; only the JS's shape differs.

- **Where the loop's turn ends at the `switch`**, the `switch` stays, its
  breaking cases `break;` it, one `break;` after it leaves the loop, and
  each case that goes on to the next turn ends in `continue;`, a value of
  none of them too, `default: continue;`, as MIR already gave React's
  `frame` (`switches.rs`' `framed`).
- **Otherwise the `match` is no `switch`**, its arms the `if` chain they
  are, `if (codes[i] === 46 || codes[i] === 33) { break; }`, the subject
  read as that chain reads it: in place where it's plain, else MIR's
  temporary named as the program names it. ADR 0958's chain of
  `js::strict_equals` tests already makes no `switch` of a chain an arm of
  which breaks (`strict_switches.rs`), so it doesn't make one again.
- **A loop the program labeled**, `'rows: for ..`, keeps its `switch` and
  `break rows;` (ADRs 0742, 0944).
- **Not yet: guarded arms that fall through (ADR 0905)** in a loop whose
  turn goes on past the `match`, `test/corpus/switch_fallthrough.rs`'s
  `counted`: MIR shares their guards' tests, and the `if` chain it gives is
  labeled blocks, `bb38: { bb15: { .. } }`, worse than the label, so that
  `switch` keeps `loop:` until tidy rebuilds such a chain; written up for
  the user (S/decisions-pending.md, #389b). Since: the amendment below.

The guide's `count` is the `if` chain. Held by
`test/lowering-switch-ends.test.ts`' "a match whose arm breaks its loop has
no label the program didn't write", `test/corpus/match_breaks_its_loop.rs`
against native Rust, `lowering-matches`' and `lowering-switches`' loops,
and mutations `loop-break-switch-labeled`, `loop-break-framed-past-the-turn`,
`framed-case-not-continued` and `framed-default-missing`.

## Amendment: guarded arms that fall through are the `if` chain they are (#389b)

The user's rule (no JS label the program didn't write) covers the case
left above: guarded arms that fall through (ADR 0905) in a loop whose turn
goes on past the `match`. Case ([0262](0262-when-rust-and-js-disagree.md)):
N, the same tests and jumps in the same order.

- **The `switch` is made as before, then taken apart** (`loop_labels.rs`'s
  `guarded_loops`, a stage after the switches' own): of a loop tidy
  labeled `loop`, which the program can't have written, each `switch` its
  cases break, its subject read as it was, is the `if` chain its arms
  are. MIR's shared guard tests and labeled blocks are settled by then,
  so the chain is built of the cases, not of MIR's chain.
- **Each arm's test is its values and those of each case that falls to
  it, and its guard where it falls**: `tag === 2 && on(two)`, then `(tag
  === 2 || tag === 3) && on(three)`, then `tag === 2 || tag === 3 || tag
  === 4`, as Rust tries `2 if .. =>`, `2 | 3 if .. =>`, `2 | 3 | 4 =>`;
  each guard runs where Rust runs it, and only there. The case's ending
  `break;` goes, and each `break loop;` and `continue loop;` is one of no
  label. The loop's label goes once nothing names it.
- **Not taken apart**: a `switch` whose subject does something, `match
  next() { .. }`, which the chain would call again for each test; it keeps
  `loop:` until the subject can be held in a `const` the names owner
  names (written up for the user; since: the amendment below). A case that leaves the `switch` from
  inside, `break;` before its end, would leave the loop in the chain, so
  that `switch` stays too; Rust gives no such case here, since only a
  label leaves a `match` early and a labeled block inside a loop needs the
  loop's label for an arm's `break`.

`test/corpus/switch_fallthrough.rs`' `counted` is now `if (tag === 1)
{ .. } else if (tag === 2 && on(log, "two", ..)) { .. } else if ((tag ===
2 || tag === 3) && on(log, "three", ..)) { break; } ..`, no label. Held by
`test/corpus/guarded_arms_leave_their_loop.rs` against native Rust,
`test/lowering-switch-ends.test.ts`' "guarded arms that fall through and
break their loop are an if chain", and mutations `guarded-loops-kept`,
`guarded-chain-forgets-fallen`, `guarded-chain-guard-dropped` and
`guarded-chain-of-a-read-that-runs`.

## Amendment: a subject held once, and MIR's shared ways out (#389c)

What the amendment above left with a label the Rust has none of. Case
([0262](0262-when-rust-and-js-disagree.md)): N each, the same tests,
calls and jumps in the same order.

- **A subject that does something is held in one `const`** (the
  coordinator's decision, applying the user's #389 rule: the #389 write-up's
  question 2). `match c.next_code() { .. }` of guarded arms that fall
  through in a tidy-labeled loop keeps the temporary MIR made of its
  subject (`loop_labels::held_subject`, which `switches`'
  `discriminant_in_place` asks before reading it in place), so the chain
  reads `const nextCode = Codes.nextCode(c);`, named by the names owner as
  every temporary is (`readable_temporaries`: what rust-analyzer suggests,
  else `value`), once a turn, as the `switch` called it. A subject MIR's
  tests read again, a place such as `codes[i]`, is read again by the chain
  too (`reads_same`, which excludes a getter): no guard can change the
  place a `match` tests, Rust's borrow of the scrutinee during its guards
  forbids it. `reads_only_vars` was the test before; its mutation goes, as
  no subject that isn't a held temporary or such a place reaches the chain.
  A subject held where no chain reads it again is read in place by the
  passes after, so `held_subject` holding one of every `switch` writes the
  same JS: it has no mutation of its own.
- **MIR's shared way out after the chain is each case's** (`switch_at`):
  of guarded arms that fall through, a `break` after their chain, the
  list's last, which each case that goes on goes on to, MIR's one way out
  of the loop or of a block, `bb42: { .. } break;`, is pushed to each such
  case, as `chain` already does for a jump after an `else`'s chain. The
  `switch` then ends where its cases do, a `break;` of the loop is `break
  loop;` and the chain the amendment above makes, with no block. Of
  `test/corpus/guarded_arms_nested_break.rs`, an arm that is a nested
  `match n { 0 => break, 1..10 => .., _ => {} }`.
- **A link's `break;` before what follows it is an `else`**
  (`elses_of_breaks`): `if (c) { a; break; } b` in a case, which an `if`
  chain's link can't write, is `if (c) { a } else { b }`; and a `break;`
  ending a branch of the `if` the case ends with goes, `switches`'
  `ending_breaks`, the rule `breaks past switches` applies to a case.
- **An arm of no statements is the next link's test**: `1 | 2 => {}` is
  `else if (t !== 1 && t !== 2) { .. }`, not an empty `then`; a last one,
  where its test does nothing, is nothing, and one whose test calls keeps
  the `switch`.
- **A tuple returned after a loop is after it** ([ADR
  0433](0433-whats-written-after-a-loop-is-after-it.md)): `exits_after`
  asks where a `return` MIR made is written by its value's span; a tuple's,
  `(i, n)`, which MIR builds of its parts, has none, so `while i <
  codes.len() { .. } (i, n)` printed `while (true) { if (i < codes.length)
  { ..; continue; } return [i, n]; }`. Its parts' spans say where it's
  written: `while (i < codes.length) { .. } return [i, n];`, as written,
  everywhere, not only after a `match`
  (`test/corpus/loop_exit_tuple.rs`).

Not yet: an arm whose statements MIR places after the chain's labeled
block, `1 | 2 => { let m = if n > 4 { 1 } else { return 0 }; .. }`
(`bb26: { bb12: { if (t === 1) { .. } if (t !== 2) { .. } } .. }`, which no
`switch` is made of); and a `match` of no guards whose arm breaks a loop
through MIR's block, `bb22: { switch (..) { case 97: break bb22; .. } ..;
continue; } break;`. Both written up for the next run.

Held by `test/corpus/guarded_arms_leave_their_loop.rs` (`polled`,
`stepped`), `guarded_arms_nested_break.rs` and `loop_exit_tuple.rs` against
native Rust, `test/lowering-switch-ends.test.ts`' "guarded arms of a subject
that runs are an if chain of it held once" and "an empty arm of guarded arms
that fall through is the next link's test", the guide's `count` of
`next_code`, and mutations `guarded-subject-not-held`, `guarded-shared-break-left-after`,
`guarded-shared-break-of-some-cases`, `guarded-chain-break-not-an-else`,
`guarded-chain-else-of-a-going-branch`, `guarded-chain-ending-breaks-kept`,
`guarded-chain-empty-link-kept` and `exits-after-tuple-unplaced`.

## Amendment: a loop left through MIR's block is left by the arm's `break` (#389c, 2026-10-29)

The second item the amendment above left: a `match` of no guards in a
loop whose turn goes on past it, an arm of which breaks the loop, `b'a' =>
break`, or a nested `match`'s, `b'd' => match n { 0 => break, .. }`. MIR
leaves the loop through a block of its own, `bb22: { if (codes[i] === 97)
{ break bb22; } ..; continue; } break;`, so `switches` saw a block's
`break`, not the loop's, built the `switch`, and printed `bb22: { switch
(..) { case 97: break bb22; .. } ..; continue; } break;`, a label the Rust
has none of.

**A loop's body that ends with a block MIR made, then the loop's `break;`,
is the block's statements, each `break` of it the loop's `break;`**
(`continues::blocks_breaking_loops`, tidy's `chains` stage after `while
tests`, before `switches`): the same jump, where the block's end went on
to. `switches` then sees the arms' `break` of the loop, and the #389 rule
makes them the `if` chain they are where the turn goes on past the
`match`: `if (codes[i] === 97) { break; } if (codes[i] === 98) { i += 2;
continue; } if (codes[i] === 100) { if (n === 0) { break; } .. }`. Not a
loop the program labeled, whose `break`s are its label's
(`case_rests::loops_left_by_cases`, which reads that shape after
`switches`); nor a block a `break` of which is in a loop or a `switch` of
its own, where `break;` would leave that one (`rows_until`: a wrong answer
otherwise, the mutation `blocks-breaking-loops-inner-loop`).

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same jumps.

Not yet: the subject read through MIR's copy of its index, `const value =
i; if (codes[value] === 97)`, where `codes[i]` reads the same (no arm's
test runs after `i` changes); the guarded arm placed after the chain's
block (`1 if on(n) => break, 1 | 2 => { let m = if n > 4 { 1 } else {
return 0 }; .. }`, `bb26: { bb12: { .. } .. }`), which needs the chain
rebuilder of #389b's write-up; and an inner loop's way out MIR shares with
the outer block, `rows_until`'s `while (true) { .. if (k) { break; } break
bb25; }`.

The readability ratchet's `labeled blocks` and `while (true)` shares grew
(1.3 to 1.4, 10.4 to 10.5 in 10,000 lines), blessed for this reason:
`rows_until` is the case that holds a block's `break` in an inner loop
apart, the wrong answer otherwise, and its `bb25:` and `while (true)` are
that open gap, above, not a shape this change makes.

Held by `test/corpus/match_breaks_its_loop.rs` (`skipped`, `rows_until`,
`shadowed`) against native Rust, and the mutations
`blocks-breaking-loops-kept`, `blocks-breaking-loops-of-written-label`
(`while_let_at_end.rs`) and `blocks-breaking-loops-inner-loop`
(`mir/tidy/jumps/continues.ts`).

## Amendment: a guarded arm placed before the arm it falls to (#389c)

A guarded arm that leaves, `1 if on(n) => break`, before the arm its
value falls to, `1 | 2 => { let m = if n > 4 { 1 } else { return 0 }; ..
}`, which MIR places after the chain's block, printed `bb26: { bb12: { if
(t === 1) { if (!on(n)) { break bb12; } break; } if (t !== 2) { .. } } ..
}`. `jumps into branches` (ADR 0745) had pushed the `break;` MIR leaves
the guard by into the chain's arms and split the chain, so `switches`
couldn't read the arms by their blocks. It now leaves alone a chain that
tests an arm of a `match` whose guarded arms fall to the next
(`switches::switched` of those `switch`es, the same hints `switches` reads), and
`switches` then `guarded_loops` make it the `if` chain Rust tries:

```js
for (const t of tags) {
  if (t === 1 && on(n)) {
    break;
  }
  if (t === 1 || t === 2) {
    if (n <= 4) {
      return 0;
    }
    ..
  } else if (t === 3) {
    ..
  }
  n = (n + 1) >>> 0;
}
```

The `switch` that stays of such a `match`, `switch_guards_apart`'s, has its
last case's `break;` that ends its guard's branch gone too, where the case
falls to nothing and no `default` follows: the case's end goes where the
`break` does (`if (!owned && on(..)) { info += "forwardRef"; }`, as it read
before).

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same jumps.

Held by `test/corpus/guarded_arm_breaks_before_its_fall.rs` against native
Rust and the mutations `jumps-into-branches-of-falling-match`
(`mir/tidy/jumps/continues.ts`) and `last-falling-case-keeps-break`
(`mir/tidy/switches.ts`).

## Amendment: the subject read as written, and a guard whose `else` leaves (#389c)

The `if` chain of guarded arms that fall through read its subject through
MIR's copy of the index, `const value = i; if (codes[value] === 97 && ..)`.
The chain's tests run one after another before any link's statements, and
no test sets a variable the function declares (a test is an expression,
which in tidy sets nothing; a call can't set a local Rust lends to the
`match`), so `codes[i]` reads the same item in each: the copy, MIR's
temporary of a variable the function declares, read by the chain's tests
alone, is that variable (`loop_labels::copy_read_in_place`). A copy of
what a call may set, a thread-local's index, `codes[AT.get()]` with a
guard that sets `AT`, stays.

And a guard MIR tests as `if (!guard) { next } else { break; } rest`, the
arm before the one it falls to leaving, `b'a' if on(n) => break` before
`b'a' | b'b' if .. => { i += 2; continue; }`, is the guard's `if` too
(`guard_alone`), so that `match` is the chain, not `bb25: { bb10: { .. }
}`:

```js
while (i < codes.length) {
  if (codes[i] === 97 && on(n)) {
    break;
  }
  if ((codes[i] === 97 || codes[i] === 98) && on((n + 1) >>> 0)) {
    i += 2;
    continue;
  }
  ..
}
```

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same item read,
the same tests in order.

Held by `test/corpus/guarded_arms_index_copy.rs` (and `loop_exit_tuple`)
against native Rust, and the mutations `copy-read-in-place-kept`
(`mir/tidy/loop_labels.ts`) and `guard-alone-else-leaving-unread`
(`mir/tidy/switches.ts`).

Amended, 2026-11-02: `guard_alone`'s arm for `if (!guard) { next } else
{ .. } rest` is gone, and its mutation `guard-alone-else-leaving-unread`
with it. By the nightly of 2026-10-31 that mutation survived:
`guarded_arms_index_copy`'s JS was the same without the arm. The arm, with
a probe that logged each time it was taken, was taken by none of the
corpus's 1,398 cases, the suite or the React port's 630 modules. The
chain above is still the case's, held by its snapshot and native Rust.

## Amendment: a `switch` holds each arm of constants (#389c)

Where an arm that leaves the loop, `b'a' => break`, is tested apart, `if
(c === 97) { break; }`, before what follows the `match`, the chain of the
arms left was a `switch` of one case, `switch (c) { case 98: .. }`. A
`switch` is now made only where each arm of constants is one of its cases;
else the arms are the `if` chain they are, `if (c === 97) { break; } if (c
=== 98) { .. }`, as #389 writes them.

Case ([0262](0262-when-rust-and-js-disagree.md)): N.

Held by `test/corpus/match_breaks_its_loop.rs` (`shadowed`) against native
Rust and the mutation `switch-missing-an-arm` (`mir/tidy/switches.ts`).

## Amendment: a name a guard compares with a read is a case of it (rd113, the user's decision)

React DOM's SimpleEventPlugin writes `case ANIMATION_END: case
ANIMATION_ITERATION: case ANIMATION_START:` of module constants that
`getVendorPrefixedEventName` makes at load. The Rust is a thread-local and
a guarded arm, `name if ANIMATION_END.with(|e| e == name) => ..`, an arm of
no constant, so the whole `match` was an `if` chain, and with a guarded arm
falling to the next (ADR 0905), a labeled block, `bb27: { .. }` (rd118).

The user decided (2026-10-29): a guard comparing what's matched with a
plain thread-local or `static` read, with no call, is a `case` of that
value; guards with calls stay guards. `hints::written_switches` makes such
an arm, a name alone whose guard is `X.with(|x| x == name)`,
`X.with_borrow(..)`, `X.get() == name` or `name == STATIC`
(`reads_compared`), one case of `Constant::Read`, written where its guard
is. tidy's chain takes its test's value where it's a read, a name, another
module's export, `use names::ANIMATION_END` as React DOM's DOMEventNames'
are (rd123), or a thread-local cell's `X.value`, calling nothing, compared by `===` with
what the tests before it compare: `switch (name) { .. case ANIMATION_END:
.. }`. A guard that calls something, `e.trim() == name`, or compares a
local or a parameter, `n if n == limit`, leaves the `match` the `if` chain
it was.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, nothing the program
can observe. A JS `switch` evaluates its cases' expressions in order,
each only where none before it matched, and compares each with `===`, as
Rust tries each arm in order and the guard's `==` was `===`. The cases
before the read are constants, so the read is made exactly where Rust
reads it, and a thread-local that changes between matches is read as it
is then. A guarded constant arm whose failed guard goes past later arms
(ADR 0636) can't be a case where such an arm follows, as that arm may
match its value: it stays a chain.

Held by `test/corpus/switch_case_of_a_read.rs` (a `RefCell` thread-local
changed between matches, to a value an earlier arm names too, a `Cell`
one's `get()`, a `static`, a guard of a call and one of a parameter)
against native Rust, and the mutations `guard-read-not-a-case`,
`guard-read-of-a-local`, `guard-read-of-any-cell` (`mir/hints.ts`), `case-read-of-a-call`,
`case-read-of-a-cell-unread` and `case-read-of-an-import-unread` (`mir/tidy/switches.ts`;
the case's `imported`, a thread-local of another module, rd123).

## Amendment: an inner loop's way out MIR shares with the outer's (#389, `rows_until`)

```rust
while r < rows.len() {
    match rows[r].len() {
        0 => break,
        2 => {
            let mut k = 0;
            while k < row.len() { if row[k] == 7 { break; } k += 1; }
            if k == 0 { break; }
            n += k as u32;
        }
        _ => {}
    }
    ..
}
```

was `bb25: { switch (row.length) { case 0: break bb25; case 2: { ..
while (true) { .. if (k) { break; } break bb25; } .. } } .. continue; }
break;`: MIR shares the outer loop's way out with the `if k == 0 {
break; }` after the inner loop, so `exits_after` moved what's after it
out, `n += k`, but left that `if` in the inner loop, `if (k === 0) { break
bb25; } else { break; }`, a jump past its own, and the outer loop's way out
a block of MIR's, which a `switch`'s `break` couldn't name.

Now `exits_after` moves a loop's last `if` the program wrote after it, of
a jump past it, before the loop's own `break`, out after the loop too
(`tested_exit_after`), where it's written: the inner loop is `while (k <
row.length) { if (row[k] === 7) { break; } k++; }`, `if (k === 0) {
break; }` after it, a `break` of the outer loop, and the `match` the `if`
chain #389 writes where the turn goes on past it:

```js
for (let r = 0; r < rows.length; r++) {
  let row = rows[r];
  if (row.length === 0) {
    break;
  }
  if (row.length === 2) {
    let k = 0;
    while (k < row.length) { if (row[k] === 7) { break; } k++; }
    if (k === 0) {
      break;
    }
    n = (n + k) >>> 0;
  }
  ..
}
```

Not where the `if` reads what the loop declares, jumps to the loop, or
the loop `break`s elsewhere, whose way past the `if` it would change.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The `if` runs once
the inner loop's way out is taken, as it did, with the same values.

Held by `test/corpus/match_breaks_its_loop.rs` (`rows_until`) against
native Rust and the mutation `tested-exit-kept-in-loop`
(`mir/tidy/jumps/tails.ts`). This closes #389: no label the program didn't
write is left in its cases.

## Amendment: the guard of a block whose `break` is in a loop of its own is held (2026-10-29)

`blocks_breaking_loops` (#389c) leaves a block MIR made in place where a
`break` of it is in a loop of its own, since a `break;` there would leave
that loop. No case reached the guard, so its mutation survived. An arm
whose inner `loop` breaks only itself, then the arm breaks the outer,

```rust
b'b' => {
    loop {
        n += 1;
        if n > limit {
            break;
        }
        ..
    }
    break;
}
```

has MIR leave the inner loop by the outer's way out, a `break` of the
block from inside the inner loop. Without the guard the block's statements
were spliced in with that `break bb19` left naming no block, which tidy's
"each jump lands" refuses in a debug build, and JS refuses as a syntax
error. With it, `case 98: while (true) { .. break; } break;` and the
`break;` after the `switch`, each leaving its own loop, as in Rust.

Case ([0262](0262-when-rust-and-js-disagree.md)): N.

Held by `test/corpus/match_breaks_its_loop.rs`'s `counted` against native
Rust and the mutation `blocks-breaking-loops-inner-loop`
(`mir/tidy/jumps/continues.ts`), now caught.

## Amendment: a `Some`'s `switch` takes `None` to its `default` (#221, 2026-10-29)

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The same arm, `None`
or a value no case names, taken.

A `match` of an `Option`'s constants whose `_` does something,
`match tag(value) { Some(A | B) => hit = true, Some(C) => return, _ =>
hit = false }`, kept MIR's block: `bb13: { if (symbolFor != null) {
switch (symbolFor) { case A: case B: hit = true; break bb13; case C:
return; } } hit = false; }`, as `None` and what no case names both reach
the `_` arm's statements MIR placed after the `if`. The `if` of the
`Some` went only where the `default` did nothing, and only of literal
cases, `"fulfilled"`.

**The statements after that `if`, in the block, are the `default`'s**,
`switch (tag(value)) { case A: case B: hit = true; break; case C: return;
default: hit = false; }`, as JS's `switch` takes `null` and `undefined`
to the `default` where no case is theirs: of literal cases, or of a
`match` whose every constant is a number, a string or the registry's
symbol (`Switch::valued`, `PatternValue`). Not where a case's `break;`
leaves the `switch` for those statements, which in the `default` it
would skip, nor where they hold a `break;` of the loop, which in the
`default` would leave the `switch`; and a `default` of its own is no
`switch` the `if`'s `else` falls to.

Held by `test/lowering-switches.test.ts`'s "a value some arms of a match
of symbols give is given where they give it" (`kept`, `continued`,
`returned`, `unwound`, `observed`, `guarded`, `stopped`, against what
native Rust gives) and `test/corpus/option_switch_default.rs`; mutations
`some-switch-default-off`, `some-switch-rest-break` (`tidy/switches.ts`)
and `switch-constants-unvalued` (`hints.ts`). A case's `break;` before
the rest has none: no tree the corpus has makes one.

## Amendment: no `tested_exit_after` (2026-10-29)

The #389 amendment's `tested_exit_after` moved an inner loop's last `if`
the program wrote after it out after the loop. Later that day the passes
before it came to write that `if` after the loop themselves, and it
decided nothing: its mutation, `tested-exit-kept-in-loop`, survived a
full check, and with it off no corpus case's JS changed (snapshots
compared), nor the snapshot, lowering, JSX, React, docs, readability,
compiler or stable suites' 1,307 tests, nor any of React's port's 221
generated files (react-in-rust 73d8e30, each crate built with and
without it, a positive control's mutation changing them). It went with
its mutation; `rows_until` is as above, held by
`test/corpus/match_breaks_its_loop.rs`'s snapshot against native Rust.

Case ([0262](0262-when-rust-and-js-disagree.md)): N.

## Amendment: no guard of a `default`'s rest that breaks its loop (2026-10-30)

The #221 amendment's `some_switch_defaults` took what MIR put after a
`Some`'s `if (x != null) { switch (x) { .. } }` for the `switch`'s
`default`, but not a rest that `break`s its loop, whose `break` a
`default` would make the `switch`'s. No program reaches that guard: the
rest of a `_` arm that breaks its loop is one the `Some`s go past, which
makes the `switch` a labeled block's, its `default` a jump, or one of
`continue`s with no `if` of its `null`, neither of the shape the pass
takes. Its mutation, `some-switch-rest-break`, survived main's checks;
with the guard off no corpus case's JS changed (snapshots compared), nor
the snapshot, lowering, JSX, React, docs, readability, compiler or stable
suites' 1,313 tests, nor any of the 222 files of React's port's crates
(react-in-rust 7cc212e, each built with and without it), nor probes of
`Option<&str>` and `Option<u32>` matches whose `_ => { ..; break; }`.
The guard went with its mutation. `test/lowering-switch-symbols.test.ts`'s
"a match of symbols whose other arm breaks its loop leaves the loop" runs
both shapes against native Rust.

Case ([0262](0262-when-rust-and-js-disagree.md)): N.

## Amendment: an untagged enum's variant of a constant is that constant (2026-10-31, rd242)

React's processLink writes `switch (rel) { case 'preload': .. case
'modulepreload': .. case 'stylesheet': .. }`. Its port matches
`js::classify(rel)`, `Kind::String("preload") => ..`, which printed an
`if` chain of `rel === "preload"`: `constants_of` named a `Some` of
constants and an untagged variant whose value is bound or not, but not an
untagged variant whose value is a constant.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. An untagged
variant's value is the value itself (ADR 0668), so `rel === "preload"`
holds exactly where `Kind::String("preload")` matches, and a `switch`'s
`case` is that `===`.

- **A variant of an untagged enum told by its kind, of one field whose
  pattern is a constant, names that constant** (`hints/constants.rs`,
  moved out of `hints.rs` whole): `switch (rel) { case "preload": .. }`.
  A number's `switch` stays in its `typeof v === "number"` test, which
  nothing removes as `===` of a string's literal does; arms of constants
  beside an arm of a kind, `Kind::Number(_)`, stay the `if` chain of
  `typeof`s, as before.

React's addValueToProperties writes its Promise case as `if (value.status
=== 'fulfilled') { .. } else if (value.status === 'rejected') { .. }`, of
two strings and the rest, three cases: its port's `match` of them is now
a `switch`, so `test/ports/performance-track-properties` and
`lowering-labels`' "a block MIR made followed by a return" write upstream's
`if` as an `if let` chain of one `let status = js::classify(..)`, which
reads `const status = value.status; if (status === "fulfilled")`.

Held by `test/lowering-strict-switches.test.ts`' "a match of an untagged
enum's constants is a switch of the value", run in JS, and mutation
`untagged-constant-no-case` (`mir/hints/constants.ts`, where the three
mutations of the moved code went too).

## Amended (2026-10-31): a `bool`'s payload test is folded too

An `Option<bool>`'s `Some(false)` is tested `b === false` now, its value
exactly (ADR 0298, the user's decision), which no `None` passes; so
`mir_payload_tests` folds it as it folds a number's, and no longer asks
which payloads it may. The mutation `payload-test-of-a-bool`, which
folded a `bool`'s while its test was `!b`, survived (main's Check) and
is gone; `option_constants_switch.rs`'s `flag` still holds the output,
`b === false ? 1 : 0`, against native Rust.

## Amended (2026-10-31): a `switch` whose arms give `true` and `false` (rd15)

React DOM's isCustomElement writes `switch (tagName) { case
'annotation-xml': .. case 'missing-glyph': return false; default: return
true; }`. The port's `match tag_name { "annotation-xml" | .. => false, _ =>
true }` printed one boolean, `return tagName !== "annotation-xml" && ..`:
tidy's `returned_branches` folded MIR's `if`s of the arms' tests, each
leaving with a `bool`, into `&&`, before `switches` saw them, as it folds
an `&&` the program wrote that MIR made branches of. **A returned `if`
chain whose first test is a written `switch`'s arm (`switched`) isn't
folded**: the `match` is the `switch` this ADR makes of any `match` of
constants, whatever its arms give. A `matches!`, which `written_switches`
never counts (its `match` is from the macro's expansion), stays the
boolean it's written as. Case N: the same tests, in the same order, give
the same value. Held by `match_of_strings_to_bools_is_a_switch.rs`
(native-compared) and the mutation `switch-of-booleans-folded`.

## Amended (2026-10-31): a `switch` inside an `if` whose first arm does nothing (rd133)

React DOM's updateProperties writes, inside `if (hasOwnProperty.call(..)
&& lastProp != null)`, `switch (propKey) { case 'checked': break; case
'value': // .. break; case 'defaultValue': .. default: .. }`. The port's
`match key.as_str() { "checked" => {}, .. }` printed `if (.. && last !=
null && key !== "checked") { if (key === "value") { .. } else if .. }`:
tidy's `and_tests` joined the first arm's test, which MIR nests in the
`if`'s, to that `if`'s condition before `switches` saw the chain. **An
`if`'s inner test that's a written `switch`'s arm (`switched`) isn't
joined to an outer test that isn't one of its own**; one that is, an
`Option`'s presence its `Some(A)` arm tests first, is joined as before.
Case N: the same tests, in the same order. Held by
`switch_of_empty_first_arms_in_an_if.rs` (native-compared) and the
mutations `switch-arm-test-joined-to-its-if` and
`switch-arm-test-in-its-own-switch-kept-apart`.

## Amendment: an arm that binds what it matched is a case (2026-11-02, rd351)

Case ([0262](0262-when-rust-and-js-disagree.md)): N. The same tests run in
the same order, and the binding is the value the `switch` tests.

React's getComponentNameFromType is upstream's `switch (type) { case
REACT_FRAGMENT_TYPE: .. case REACT_VIEW_TRANSITION_TYPE: if
(enableViewTransition) { .. } // Fall through case
REACT_TRACING_MARKER_TYPE: .. }`. Its port writes the fall-through as one
arm, `Some(symbol @ (REACT_VIEW_TRANSITION_TYPE |
REACT_TRACING_MARKER_TYPE)) => { if symbol == REACT_VIEW_TRANSITION_TYPE
&& .. { .. } .. }`. Because of that binding, the whole `match` printed as
an `if` chain, so the port copied the tracing marker's body into the view
transition's arm instead. **A binding of constants, `symbol @ (A | B)`,
names the constants its pattern names** (`valued_constants`), so its arm
is a case like any other, `case A: case B:`. Its binding reads as what
the `switch` tests, `if (kind === A && ..)`, the same way it reads
without one. A binding of anything that isn't constants is still no
case.

Held by the corpus case `switch_arm_binding_constants` against native
Rust, and the mutation `binding-of-constants-unswitched`
(`lower/mir/hints/constants.ts`).

## Amendment: a guarded arm that only continues is its case's `break;` (2026-11-03, #488)

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each way out of the
case still goes past the `switch`, and the guard is still evaluated.

`for &x in v { match x { 0 => { ..; continue; } 1 if n > 3 => continue,
2 => n += 1, _ => {} } }` printed `case 1: if (n > 3) { break; } break;`.
The `continue` MIR made ends the loop's turn, so it was the `switch`'s
`break` (rd136), and the guard failing goes to `_ => {}`, which does
nothing. Either way, the case goes past the `switch`. Only the last case
had its `if`'s ending `break;`s taken off (#389c). **Each clause that
goes on past the `switch`, a case that doesn't fall and the `default`,
has its own `break;` and each `break;` MIR made that ends a branch of the
`if` it ends with taken off** (`ending_breaks`), as the clause's end goes
there anyway. The printer still ends a case that goes on with its
`break;`. An `if` left with nothing is gone, and one whose guard does
something is that call, as `emptied_branch` writes it:

```js
case 1:
  break;
```

and `case 1: noted(log); break;` of `1 if noted(log) => continue`. A last
case that's then nothing, `3 if n > 1 => continue` before `_ => {}`, is
no case. Its values are constants, and a value no case takes goes past
the `switch` too.

A `break` the program wrote stays where it is, `break 'get_tag;` of ADR
0944's labeled `switch`, so its `if` has no `else`, as written
(`unwritten_else`). Before, the last case's written breaks went too, but
nothing in the checks wrote one there. The last case's own rule for a
case that falls to nothing (#389c) is gone with its mutation,
`last-falling-case-keeps-break`. A probe under `check:fast --all` found
no last case that falls once tidy reaches it, so the one rule covers
what that rule did.

Held by the corpus case `guarded_continue_ending_turn` against native
Rust, `lowering-labels.test.ts`'s "a label the program wrote is the
switch's whose inner switch leaves it", and the mutations
`case-ending-breaks-kept`, `case-ending-written-breaks-dropped` and
`empty-last-case-kept` (`lower/mir/tidy/switches.ts`).

## Amendment: an arm's `continue` at the turn's end stays `break;` (2026-11-03, the user's decision, #518)

ADR 0684's #518 amendment keeps an arm's `continue` where more of the turn
follows the `match` (`case "SCRIPT": continue;`). Of a `match` that ends
the turn, the user's 2026-10-29 decision to keep each jump the program
wrote, though redundant, could have kept it too: `case 0: continue;`.
**The user decided to keep `case 0: break;`** for such an arm, as this
ADR wrote it: the `switch` goes on to the turn's end anyway, and `break;`
is what JS writes there. `switch_tail_continues.rs` and
`switch_arms_continue.rs`'s `at_the_end` hold it, with the mutation
`arm-continues-written-at-the-turns-end`.

## Amendment: every case that goes on ends with `break;`, the last too (2026-11-03, the user's decision)

The amendment "a `break` only where a case goes on to another" left out
the `break;` of a `switch`'s last clause, a last case or a `default`,
because JS falls out of it anyway. Upstream React writes that `break;`.
In the reconciler and the server (`packages/react-reconciler/src`,
`packages/react-server/src`), 29 of the 43 last `default`s that go on end
with `break;`, and so do 26 of the 39 last cases. Control-flow run 46
counted about 42 hunks in the port's ten largest files where upstream has
that `break;` and the port has none. **The user decided (2026-11-03) that
every clause that goes on ends with `break;`, the last included, in one
form.**

- `to_oxc`'s `case_body` ends a clause with `break;` wherever it goes on
  (`goes_on`). It adds none where the clause falls through to the next
  (ADR 0905) or where its last statement leaves (`js::ends`). The last
  clause is no exception. `commented_clause` places only a last clause
  that has no `break;`, whose comments then end it (ADR 0753, amended).
  Where it has a `break;`, the `break;` comes after them, as in any case.
- Tidy is unchanged: `breaks_past_switches` still removes the `break` it
  made at a clause's end, and the printer writes the one `break;`
  (#512).

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same JS. A
`break;` at the end of the last clause goes where the clause's end goes.

- The corpus and readability snapshots are re-blessed. Each change is a
  `break;` added at the end of a last case or a `default`.
- `test/lowering-switch-ends.test.ts`: "a case needs no break where it
  leaves, and the last that goes on has one" and "a default after a
  guarded case ends with one break", run.
- Mutations `last-case-unbroken` and `default-unbroken`
  (`scripts/mutations/to_oxc/statements.ts`). They replace
  `last-case-broken`, which held the old rule.
- `commented_clause` is gone. A clause that ends with comments always
  goes on, so it has a `break;`, and its comments print before it; the
  function only ever returned the place it was given. Its mutation,
  `last-clause-comments-unflushed` (ADR 0753), went with it. Tidy's
  `case-own-break-kept` (ADR 0905, #512) is now an equivalent mutant: a
  `break;` that tidy leaves at a clause's end is the one the printer
  would write. The mutant is removed; the pass, which shapes the bodies
  that the passes after it read, stays.
- *Since (2026-11-04, the user's decision, red-main run):* `to_oxc`'s
  `written_apart` no longer leaves out an arm of only comments, `3 => {
  // .. }`. It left them out so their comments, the last clause's only
  statements, weren't lost where the case was printed at its value; now
  a `break;` follows them, so they stay. A comment above such an arm is
  above its `case`, as rd619 has it above any arm that opens a block, not
  inside the case before the arm's own: `// Above the last case.\ncase
  3:\n  // The last case ..\n  break;`. No corpus case has one (the JS of
  all 1,572 files is the same). Held by `test/lowering-comments-ends.test.ts`'s
  "a last arm of only comments keeps them in its clause"; its mutation
  `commented-arm-written-apart` goes with the guard, and
  `commented-arm-not-written-apart` puts it back. The same `break;` left
  `default-pattern-comment-after-statement` reached only by a `default:`
  of one statement that leaves, `_ /* "{" */ => continue`, which
  `test/lowering-comments-jumps.test.ts` now holds. With the guard gone,
  tidy's filter of a comment above an arm that does nothing out of its
  case's body (`tidy/switches.rs`, #508, ADR 0753) changed nothing either,
  as the case is printed where its value is, the comment above it: the
  corpus's JS (1,575 files) and five probes, `1 => {}`, `1 => ()`, `1 | 2
  => {}`, a guarded and a text arm, are the same without it. The filter
  is gone, and its mutation `commented-case-in-its-body` with it;
  lowering-comments-jumps' "a comment above an arm that does nothing is
  the arm's" still holds the output, and `commented-arm-not-written-apart`
  the printer's rule.

## Amendment: an `Option<bool>`'s `Some(false)` and `Some(true)` are a `switch`'s cases (#545, 2026-11-04, the user's decision)

React's updateClassComponent writes `switch (shouldError(workInProgress))
{ case false: .. case true: .. }`. The port's `match should_error(wip) {
Some(false) => .., Some(true) => .., None => {} }` was a `const value`,
a `value != null` test and an `if`/`else` of the two, as a `bool`'s
constant names no case (two values are an `if`). The user chose the
`switch`, as an `Option<u32>`'s literals already are (2026-11-03): **a
`match` of an `Option<bool>` whose first two arms are `Some(false)` and
`Some(true)`, in either order, with no guard, and whose last, `None`, `_`
or a name, takes only `None`, is a `switch` of the value**, each `Some` a
case of its value; MIR tests the first of them, and the other is its
`else`, a variant's last case (`Constant::Variant`,
`FnCx::bool_option_arms`, `Switch::none`). The last arm is no case where
it does nothing, as `null` matches none (`breaks_past_switches` drops the
`value != null` test, as of any `Option`'s `switch`), and the `default`
where it does something, `if (x != null) { switch (x) { .. } } else {
none }`'s `switch (x) { .. default: none }` (`nones_defaulted`): both
values are cases, so only `null` reaches it. Not where `none` is a loop's
`break;`, which would leave the `switch` there; MIR then tests `None`
first, `if (value == null) { break; } switch (value) { .. }`. A `return`
of the `default`'s is after the `switch` as before (ADR 0905, amended).

An `Option` of a number, whose `Some`s name a few of its values, keeps an
arm of `None` an `if` of `value == null` before the others (the
coordinator's call on the user's behalf): a `default` there would take
each value no case names too.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same tests.
Held by corpus `switch_of_an_optional_bool.rs` against native Rust
(`None => {}`, `None` with a body, the `Some`s in either order, `_` of
returns, a guarded `Some`, whose `_` takes a value too, a loop's `break` and `continue` of `None`, and an `Option<u32>`'s
`None` arm, an `if`), `option_bool_exact.js` re-blessed, the guide's
basics.md, the unit test `a_none_is_a_default_but_of_a_break`
(`src/lower/mir/tidy.rs`), and the mutations `bool-option-no-switch`,
`bool-option-guarded-switch` (`hints/constants.ts`) and `none-undefaulted`
(`switches.ts`). No program has a `None` of a `break;` in the `else`, as
the branch that leaves is the `if` first (ADR 0237), so the unit test alone
holds that guard.

## Amendment: a match's comments, each kept (#574)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments only.
Three of a `match`'s comments were lost or moved. **A comment above an
arm of the unit value, `1 => (),`, is above its `case`**, as above `1 =>
{}`: the arm does nothing (`opens_or_guards`). **A block comment on a `_`
pattern, `_ /* "{" */ => {`, of no `default:`**, the `match` an `if` and
its arm the `else`, or the `match` gone, **is on a line of its own before
the arm's first code**, as one above the arm is (`of_no_default`); of a
`default:`, its arm on its line, `_ /* "x" */ => panic!(..)`, after the
`default:` as before, found by the arm's code on the comment's line too.
**A comment above a guarded arm that does nothing, `1 if skip => {}`,
whose test the next arm's `if` is, `else if (n !== 1 || !skip)`, is before
that test**, the merged test placed where the guard is (`and_tests`, of a
`Written::guards` guard), so in the `else`'s block, which Cx::commented
keeps (ADR 0444, amended). A comment in an arm before only a `continue`,
`1 => { // Skipped. continue; }`, is still above the `switch`: the
`continue` MIR's goto has no span of its own (#574 stays open for it).

Held by corpus `match_arm_comments_kept`, and the mutations
`unit-arm-not-apart`, `of-no-default-never`, `of-default-same-line-unread`
and `guarded-test-span-kept`.

## Amendment: a comment before an arm's only `continue` (#574)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments only.
The last of #574's comments: `1 => { // Skipped. continue; }` printed
`// Skipped.` above the `switch`. Optimized MIR has no block of the arm's:
its test of the pattern goes to the loop's start itself, so the JS
`continue` had no place a comment could be before. **The `continue` of the
`if` that tests an arm that's a block ending in one, where that `if`
starts where the arm does, is placed at the `continue` the arm writes**
(`arm_continues_placed`, of `Written::arm_continues`, in tidy's jumps
stage), and the comment is before it: in its `case`, or in its `if`'s
block. Placed, it reads as a `continue` the program wrote, which keeps its
`if (k === 3) { continue; }` as written, not `if (k !== 3) { .. }`, as
`continued` keeps one; and it's the arm's code, where `decisions` places
an `else if`, as of any other arm: a comment first in a later arm's block
nests its chain, `} else { // c\n if (..) {`, of any arm (#595). An arm
that's a `continue` alone, `(2, 2) => continue`, has nothing in it a
comment could be before, and stays as it was, MIR's, `} else if (..) {
continue; } else {`; so does a `continue` of a loop inside the arm.

Held by corpus `comment_before_an_arms_only_continue`, and the mutations
`arm-only-continue-unplaced`, `arm-only-continue-of-any-if` and
`arm-only-continue-of-a-bare-arm`.

## Amendment: a comment above an arm that's a `continue` alone (2026-11-04, the user's decision, #610)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments only.
An arm that's a `continue` alone with comments above it, `// Skipped.\n 1
=> continue,`, printed them above the `switch`, above the `if` tidy
inverted, `// c\n if (k !== 3) { .. }`, or above the `for`. The user's
decision (2026-11-04, option B): **a commented arm that's a `continue`
alone keeps its `continue` where it's written**, as the arm of a block
does already: `case 1:\n // Skipped.\n continue;`, as React DOM's
`case 'ref':\n // Noop\n continue;` is written
(packages/react-dom-bindings/src/client/ReactDOMComponent.js:2513-2515);
`if (k === 3) { // c\n continue; }`; `if (x == null) { // None is
skipped.\n continue; } log.push(x);`. One with no comment above it is the
`if` of the rest, as before: the comments, not the arm, are what has
no place otherwise.

Which arms are commented is the sources' to say (`sources::commented_arms`,
which finds an arm of nothing with comments above it, #508): each arm
whose body is a `continue`, past only comments since the arm before or
the `match`'s `{`, and not guarded; tidy's `Hints::commented`
carries them to `arm_continues_placed`. And an arm MIR tests where the
`match` is, not where its pattern is, `Some(n) => *n, None => continue`'s
`if (x == null) { continue; }` at `match x`: where it's the only arm of the
`match` that continues and the `if` does nothing else, the `continue` is
that arm's, a block's, `None => { // c\n continue; }`, as well. (That it's the only one is held by no program: MIR makes the
arms of more one block their `break`s go to, which `continues_written`
reads, below; it says which arm the `continue` is of, never another's.) Of a
`match` whose arms of `continue` MIR joins, each a `break` to the
`continue` after their block (`continues_written`), a commented arm's is
placed where its test is, its pattern, where the `if` is at the `match`:
`case 1:\n // One is skipped.\n continue; .. case 3:\n // Three is
skipped.\n continue;`.

A last arm of no test of its own, `3 => log.push(3), // c\n _ =>
continue,`, is the `else` of the `if` MIR made of the arm before, whose
branch holds a copy of what follows the `match`, so the `else` ends the
turn: its `continue` is kept there too, where `tail continues` takes one
that only ends the turn (ADR 0684), `if (k === 3) { log.push(3);
log.push(400); } else { // c\n continue; }`.

Held by corpus `comment_before_a_bare_continue_arm` against native Rust,
and the mutations
`continue-at-the-match-unplaced`,
`continue-at-the-match-uncommented`, `last-arm-continue-unplaced`,
`commented-tail-continue-taken`, `shared-continue-arm-at-the-match`
(`mir/tidy/jumps/continues.ts`),
`commented-continue-arms-unfound` and
`commented-continue-arms-of-no-comment` (`lower/sources.ts`).

## Amendment: a block of only a commented `continue` (#666)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments only. A
last arm that's a block of only a `continue`, `_ => { // c\n continue;
}`, printed its comment above the `for`: its `continue` wasn't among
`commented_arms`' continues, so `tail continues` took the `else` that
ended the turn, as it takes one with no comment. Now `commented_arms`
finds it too: a block whose only statement, or value, is a `continue`,
with comments above the arm or in the block before it, nothing else
there. Its `continue` is kept where it's written as a bare arm's is, `}
else { // c\n continue; }`. One with no comment is the `if` of the rest,
its `else` gone, as before.

`arm_continues_placed`'s check that a `match`'s continuing arm is its only
one is gone. It was held by no program, as the amendment above said: the
whole corpus compiled with and without it prints the same, and so do
probes of two arms of `continue` in one `match` (a guarded one beside
`None => continue`, one after a statement, a block's, two last arms). MIR
makes such arms one block their `break`s go to, `continues_written`'s,
so the `if` it tests at the `match` is one arm's.

Held by corpus `comment_before_a_bare_continue_arm` (`last_block`,
`above_block`, `uncommented_block`) against native Rust, and the
mutations `commented-continue-blocks-unfound`,
`commented-continue-blocks-of-no-comment`,
`commented-continue-blocks-above-only`,
`commented-continue-blocks-in-only` and `arms-of-nothing-of-no-comment`
(`lower/sources.ts`); `continue-at-the-match-uncommented`'s find moved
(`mir/tidy/jumps/continues.ts`).
