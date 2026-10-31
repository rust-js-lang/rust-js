# 0684. A `continue` the program wrote is where it wrote it, with its comment

Status: Accepted. Extends [0444](0444-a-comment-is-where-its-code-is.md),
whose comments are before the statement whose code follows them, and
[0575](0575-an-if-keeps-its-else-and-a-tail-if-is-a-conditional.md)'s
`Written::continues`. Amended by
[0730](0730-a-loops-early-break-is-in-the-if-it-was-written-in.md): a loop
body's last `if` is an early `continue` only where the program wrote one.
Amended below: a loop's `break` the program wrote is where it wrote it too
(#298); and a `for…in`'s guards, with their comments, as React's
hydration diffs write them; and (2026-10-29, the user's decision) each
`continue` and loop `break` the program wrote is kept, though the turn or
the function ends there anyway.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. How the JS reads, and
a comment, which nothing runs.

## Context

React's `isSimpleObject` (shared/ReactSerializationErrors.js), ported,
lost a comment and its `if`:

```js
// The original:                               // rust-js's:
if (!descriptor.enumerable) {                  if (!(descriptor.enumerable ?? false)) {
  if (                                           if ((name !== 'key' && name !== 'ref')
    (name === 'key' || name === 'ref') &&            || !descriptor.get) {
    typeof descriptor.get === 'function'           return false;
  ) {                                            }
    // React adds key and ref getters ..       }
    continue;
  }
  return false;
}
```

MIR has `if (a) { if (b) { continue; } } return false;`. Tidy's rule for
a `let … else`, `if (a) { if (b) { rest } } return f;` as `if (!(a && b))
{ return f; } rest`, took the `continue` for the rest it guards: the `if`
turned, its `continue` then the loop's end, gone. And the `continue`
itself, a jump MIR makes back to the loop's start, has no place in the
Rust: MIR's own spans don't keep where the program wrote it once its
blocks are merged, so the comment before it had no statement to be
before.

## Decision

**An `if` the program wrote of a `continue` is as written**, not a `let
… else`'s guard: THIR says where each is (`Written::continues`), and the
rule leaves an `if` of its test alone, as it leaves one of a `return` the
program wrote.

**A `continue` the program wrote is where it wrote it**: each of
`Written::continues` carries where its `continue` is as well as its
test, and the last pass, `written jumps`, gives that place to the
`continue` its `if` ends with, so its comment is before it, as any
statement's is.

## Why

- **It's how it's written**, in the Rust and in React's JS: a test, a
  comment that says why, and a `continue`.
- **One account of what the program wrote of a `continue`**: the hint
  tidy's `continue` passes already read says where its `continue` is too.

## Alternatives

- **The `continue`'s place from MIR**, as a `return`'s is found back
  through the drops on its way: the block MIR jumps from is merged into
  the `if`'s, its terminator of no place. Tried; rejected.
- **A comment before a jump moved to the `if`**: not where it's written.
  Rejected.

## Consequences

- One corpus case reads as written: `returns_in_place`'s `firstEven`,
  `if (n % 2 === 1) { continue; } return { TAG: "Ok", _0: n };`, where it
  was `if (n % 2 !== 1) { return ..; } continue;`. No snapshot changed.
- `tail_continues`, which drops a `continue` where a loop's body ends
  anyway, no longer keeps one the program wrote there: what kept it was
  this turned guard's `continue`, which no longer is; one the program
  wrote last in a body does nothing in JS either. Its mutation,
  `tail-continues-written-dropped`, goes with it.

## What holds it

- `test/lowering-written-branches.test.ts`, "a continue the program wrote
  keeps its if, its test and its comment": isSimpleObject's shape, run.
- Mutations: `written-continue-guarded`, `written-continue-unplaced`.

## Amendment: an `else` of only the loop's `continue` is no `else`

**Where a loop's body ends in `if (t) { ..; return; } else { continue;
}`, the `continue` MIR wrote for the turn's end, the `if` has no
`else`**: `tail_continues` dropped such a `continue` only where it
followed something in its list, and preparation then left it past the
`return`, a `continue;` the program didn't write, as React's
`isSimpleObject` read once its `let … else` was a guard. Now it ends
`return false; } }`, as React's does.

- `test/lowering-written-branches.test.ts`, "a loop's body that ends in an
  if of a continue has no continue after it", of the js crate's
  reflection, run; mutation `else-continue-kept`.

## Amendment: a loop's `break` the program wrote is where it wrote it too (#298)

React's scheduler's `workLoop`, ported, writes a comment before each of
its loop's two `break`s:

```rust
while let Some(current_task) = CURRENT_TASK.with_borrow(Clone::clone) {
    if !ENABLE_ALWAYS_YIELD_SCHEDULER.with(|&always| always)
        && current_task.expiration_time > current_time
        && should_yield_to_host()
    {
        // This currentTask hasn't expired, and we've reached the deadline.
        break;
    }
    ..
}
```

Both printed above `while (true)`, where upstream has each in its `if`
before its `break;`. A loop's `break`, a jump MIR makes out of the loop,
had no place of its own: only a labeled block's `break` had the one it's
written at (ADR 0790), and the passes that turn a loop's `if`s rebuild
its `break` at the `if`'s. So the comment, whose next statement is the
first that starts after it, was the loop's.

**A loop's `break` the program wrote, at the end of an `if`'s `then`, is
where it wrote it, as a `continue` is**:

- `Written::breaks`, as `Written::continues`, says where each `if` the
  program wrote of a loop's `break` tests and where the `break` is: one
  THIR walk, `hints::jump_ifs`, of the `if`s and the `let … else`s of a
  jump, answers both, each by its kind of jump. A labeled block's
  `break`s, whose places ADR 0790 gives, aren't of it.
- The last pass, `written jumps` (`written continues` before), gives that
  place to the `break` an `if` of it ends its `then` with, so its comment
  is before it. An `if` is the program's where it's at its test, or at
  the `if` it's written in, which ends where its test does
  (`jump_tested`), `if i > n`; or where the loop's own test is joined to
  it, `if (x == null || (stop && x > 3))` of a `while let`, where its last
  `||`'s operand is the program's test. A test of no place, as tidy joins
  or reads one, `!enableAlwaysYieldScheduler && ..` of a thread-local's
  read, is where the first of what it's made of is (`written_at`).

The scheduler's two comments are before their `break`s, as upstream's.
No corpus case or other snapshot changed.

Case: N, as above. A comment, and where it is, which nothing runs.

- `test/lowering-comments.test.ts`, "a comment before a loop's break is
  before the break": a `while let`'s, a `loop`'s two and a `while`'s, run;
  `test/port-scheduler.test.ts`, the scheduler's.
- Mutations `mir-loop-breaks-of-blocks`, `written-break-unplaced`,
  `written-break-or-unread`, `jump-tested-at-its-test-alone` and
  `written-at-its-own-span`.

## Amendment: a `for…in`'s guards, as React's hydration diffs write them

React's `ReactFiberHydrationDiffs`, ported, writes each `for (const
propName in props)` as upstream does, two `continue`s first:

```rust
for prop_name in js::keys_in(props) {
    if !object::has_own_property.call(props, prop_name.as_str()) {
        continue;
    }
    if prop_name == "children" {
        // Ignored.
        continue;
    }
    ..
}
```

Its JS read four ways upstream's doesn't:

- **`// Ignored.` before the `for`.** `continued`, which puts back an
  `if` of a `continue` the program wrote, gave the `continue` the `if`'s
  place, so `written jumps`, which places only a `continue` of no place,
  left it at the `if`, before its comment, and the comment found no
  statement in the loop to be before. **A `continue` `continued` puts
  back has no place of its own**: `written jumps` gives it where the
  program wrote it, its comment before it.
- **The two guards one `if (a && b) { ..; continue; }`**, in a loop that
  `break`s: MIR ends each turn with a `continue` of the loop, which the
  `break`'s block leaves after the guards' `if`, so `continued` found no
  `if` last and put none back. **MIR's `continue` that ends a turn, after
  an `if` of one the program wrote, goes** (with what can't follow it),
  and the guards are put back as in a loop with no `break`.
- **A comment inside a let-chain's test before the `for`**, `// Only do
  the diff ..` before `&& (b.len() > 2 || ..)`: the operand, placed
  nowhere, started where the first of its parts did, and `a` of `if let
  Some(a) = a` reads where its pattern binds it, before the comment, so
  the comment was before no start. **A part placed is all of it**: what's
  read inside it isn't where it is (`to_oxc.rs`'s `written_extent`), and
  the comment is before its operand, as upstream has it (ADR 0444).
- **`let propName;` before `for (let propName in value)`**: a `for mut`
  binding is declared first, for MIR's assignment each turn, and `loops`
  takes that `let` only where it's beside the loop. **A `let` of no value
  a function declares that only loops binding it now name goes**
  (`loop_bound_lets`), wherever in the function the loop is.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. A `continue` that
ends a turn goes where the turn's end does; a `let` nothing reads, and
comments, run nothing.

- No corpus case's or snapshot's JS changed but the new one's.
- `test/lowering-written-branches.test.ts`, "a for…in's continues are as
  written, their comments in place, its mut binding its own", run;
  `test/corpus/continues_before_break.rs` against native Rust.
- Mutations `restored-continue-at-its-if`, `loop-tail-continue-kept`
  (`tidy/jumps/continues.ts`), `loop-bound-let-kept` (`tidy/loops.ts`)
  and `written-extent-of-a-binding` (`to_oxc.ts`).

## Amendment: a `break` of the loop a function ends with is a `break` (2026-10-27)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same jump: past
the loop a function ends with is its end.

React's commitDeletedPairViewTransitions, `while (child !== null) { .. if
(pairs.size === 0) { break; } .. }`, the loop its function ends with
(cv5 of the port's gaps). MIR's jump of the `break` goes to the function's
end, the `return` its loop's end goes to, and the JS was `if (..) {
return; }`.

**In the loop a function ends with, an `if` the program wrote of a
`break` (`Written::breaks`) whose `then` ends `return;` ends `break;`**
(`continues::written_breaks_of_last_loops`), as written; not in a loop or
a `switch` inside it, which a `break;` would leave instead, and not a
`return` the program wrote.

Held by `test/lowering-early-exits.test.ts`, "a break of the loop a
function ends with is a break", run, beside a written `return`; the
mutations `last-loop-break-returned` and `last-loop-return-of-any-if-broken`
(`mir/tidy/jumps/continues.ts`).

## Amended (2026-10-28): no loop's last `continue` to take off

`continued` took off MIR's `continue` that ends a loop's turn after an
`if` of one the program wrote, where the loop also breaks. No corpus
case, lowering, port or JSX test reaches it now, nor a probe of the
shapes it was for (a `break` then an `if` of a `continue` ending the
body): what's before reads such a turn's end as the body's end. It's
gone with its mutation, `loop-tail-continue-kept` (ADR 0093).

## Amendment: a jump the program wrote is kept, though redundant (2026-10-29, the user's decision)

React DOM's replayUnblockedFormActions, ported (rd109), writes upstream's

```rust
if action < 0 {
    let blocked_on = blocked(-action);
    if blocked_on.is_none() {
        // Not blocked: skip past it.
        i += 3;
        continue;
    } else {
        // We're blocked. We'll retry later.
        break;
    }
}
let inst = ours(action);
if let Some(inst) = inst {
    ..;
    // Continue without incrementing the index.
    continue;
}
i += 3;
```

in a loop its function ends with. The JS was `if (blockedOn == null) { i
+= 3; } else { return; }`, the rest an `else` of `if (action < 0)`, and
`// Continue ..` ended a branch with no `continue` after it: MIR has the
`break` as the function's `return;` (`written_breaks_of_last_loops` turned
only a `then`'s back), and each `continue` the loop's end, which tidy
takes where the turn ends anyway (`tail jumps`, `tail continues`).

**The user decided: keep the `continue` and `break` the program wrote,
even where redundant**, as a written `else` (ADR 0237, amended) and a
written label (wl4) are kept. So:

- `hints::jump_ifs` says where each `if … else` whose `else` ends in a
  `continue` or a loop's `break` is, `Written::else_continues` and
  `else_breaks`, beside the `then`s' `continues` and `breaks`.
- **`kept_jumps`, the first part of `written jumps`, now in `finish`**,
  where nothing moves a jump after it: in each loop, a branch of an `if`
  of one of those that the turn ends after, and that doesn't end in a
  jump, ends in the `continue` again, at the place it's written, its
  comment before it; and in the loop a function ends with, a branch's
  `return;` of an `if` of a written `break`, `then`'s or `else`'s, is that
  `break` (`written_breaks_of_last_loops`, which it replaces). Not a
  branch whose statements are written after the jump, the other of an
  `if` tidy turned (`std_enumerate.rs`'s), nor in a loop or `switch`
  inside the loop, nor where a drop follows the `if`, `if (x > 3) { .. }
  else { .. } drop(n)`, whose turn doesn't end after the branch.
- Preparation's `unelsed` (ADR 0237) then puts what MIR made the `else`
  of an `if` written without one after it again, as its `then` now ends:
  `if (action < 0) { .. } const inst = ..`.
- `written elses` turns back an `if` of MIR's `return;` of such a `break`
  too, `if (x <= 3) { return; } out.push(x);` of `if x > 3 { .. } else {
  break; }`, as it turns one of a lone `break` or `continue`.
- `tail continues` leaves an `else` the program wrote where it was, `if
  (x > 3) { ..; continue; } else { .. }`, not after the `if`.

The port's JS is upstream's, and `normalizeConsoleFormat`'s `if (..) {
i++; continue; }`. In the corpus, `loop_drops.rs`'s `} else { ..;
console.log("moved"); continue; }` reads as written.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. A jump where the
turn or the function ends anyway does what falling there does.

Not covered yet: a `continue` in a `match` arm a `switch` is made of, whose
case ends in the `switch`'s `break;` (`switch_tail_continues.rs`'s shape),
and one before a drop, which MIR shares after the `if`.

- `test/lowering-comments-jumps.test.ts`, "a comment before a break or
  continue tidy takes is where it was written", run; the corpus case
  `written_jumps_kept.rs` against native Rust.
- Mutations `mir-else-continues-of-thens`, `mir-else-breaks-unknown`
  (`hints.ts`), `kept-last-loop-breaks-unrun`, `kept-continue-dropped`,
  `kept-continue-before-the-turn-end`, `kept-else-jumps-of-thens`,
  `kept-break-of-a-turned-if`, `written-continue-else-dropped`, and
  `last-loop-break-returned` and `last-loop-return-of-any-if-broken` moved
  to `kept_jumps` (`continues.ts`); `written-else-of-a-made-return-unturned`
  (`tails.ts`).

### And a guard's `continue` where it's written (rd152)

`kept_jumps` gave a written `continue` back to the `then` of `if (!c) {
rest }`, MIR's turn of `if c { continue; } rest` in an arm, as the end of
a branch the turn ends after: `if (!isHiddenSubtree(node)) { ..; continue;
}` in React's findAllNodes, `if (!warnedAbout[key]) { ..; continue; }` in
validateShorthandPropertyCollisionInDev. **Such an `if`, where the turn
ends after it and its `then` is all written after the jump, is turned
back, `if (c) { continue; } rest`** (`turned_continue`), the one rule
`continued` applied to a loop body's last alone, which now calls it
first and `kept_jumps` everywhere a turn ends; a statement written across
the jump, `node = { ..; if c { continue; } node }`'s assignment, is after
it. One the program wrote last in its `then`, `if x > 3 { out.push(x);
continue; }`, is no longer turned, `if (x <= 3) { continue; }
out.push(x);`, but kept, its `continue` given back. A rest of a second
`let` of a name declares it apart, `doubled$1`, so moving it out of the
`then` clashes with nothing.

React DOM's restoreControlledInputState now prints the port's `if
(otherNode === rootNode || otherNode.form != rootNode.form) { continue; }`
as its Rust writes it, one `if` of `||`, where it printed two (upstream
writes two `if`s; the port's Rust, one).

- `written_jumps_kept.rs`'s `roots`, `shadowed` and `last_continue`,
  against native Rust; mutations `kept-turned-continue-unturned`,
  `kept-turned-continue-of-any-then`, and `mir-tidy-continue-guard-kept`,
  `mir-tidy-written-continue-nested`, `mir-tidy-continue-written-negated-only`
  and `restored-continue-at-its-if` moved to `turned_continue`.

## Amendment: no `||` the loop's test joins is read for the program's `if` (2026-10-29)

The #298 amendment read an `if` whose test the loop's own joined, `if (x
== null || (stop && x > 3))` of a `while let`, as the program's `if` of its
jump by its last `||`'s operand (`written_jump`). Since ADR 0702's #301
amendment a `while let`'s `Some` is the loop's test, apart from the
program's `if`, so no such join is made: the mutation of that reading,
`written-break-or-unread`, survived a full check, and with the whole
reading out no corpus case's JS changes (snapshots compared), nor the
lowering, JSX, React, port or snapshot suites'. The reading went with its
mutation; an `if` is the program's by `jump_tested` alone. Case
([0262](0262-when-rust-and-js-disagree.md)): N, a comment's place.

## Amendment: a written `continue`'s `if` before more of the turn (2026-10-30)

The mutation `written-continue-guarded`, `returned_branches`' check of a
written `continue` taken out, survived the nightly: its test's `if` ends
the loop's turn, where `kept_jumps`' `turned_continue` (`finish`) turns the
guard back, `if (c) { continue; } return false;`, as written. Before more
of the turn, `if c { // ..\n continue; } return None; } count += 1;`, no
pass turns it back, and the check is what keeps it: without it, `if (!c)
{ return; } continue;`, its comment moved. The test "a continue the
program wrote keeps its if" has such a loop now, `counted`, run, and
catches the mutation. Case ([0262](0262-when-rust-and-js-disagree.md)):
N, a comment's place.

## Amendment: a loop's `break` where a branch of the function's last `if` ends with the loop (rd165, 2026-10-31)

A loop's `break` the program wrote is `break;` in the loop a function ends
with, where MIR has its `return;`. React DOM's warnForBlockInsideInline
ends with `if (__DEV__) { .. while (nextNode != null) { .. if (..) { ..;
break; } .. if (node === instance) { break; } .. } }`, its loop the last
of the `if`'s branch, and its port printed both `break`s as `return;`:
the function ends after that branch too, so MIR's jump there is the
function's `return`, but `kept_jumps` looked at the function's last
statement alone. It now looks through an `if` that ends the function, each
branch whose last statement is a loop or another such `if` (`ending_loops`),
and gives each `break` the program wrote there back: `if (dev) { while
(next != null) { if (next > 3) { out.push(next); break; } .. } } else {
for (const x of xs) { if (x === 0) { break; } .. } }`. Case
([0262](0262-when-rust-and-js-disagree.md)): N, a `break` where nothing
follows it but the function's end. Held by the corpus case
`written_jumps_kept` (`in_dev`) and the mutations
`kept-breaks-of-a-branchs-loop-unrun` and
`kept-breaks-of-an-elses-loop-unrun`.
