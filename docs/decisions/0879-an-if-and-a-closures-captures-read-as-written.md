# 0879. An `if` reads as written, and a closure's captures are read in place

Status: Accepted. Amends [0237](0237-no-else-after-leaving.md)'s scope (an
`else` that leaves a loop the program wrote stays),
[0364](0364-mir-lowering.md)'s fold of a branch of `false` (an `if`'s value
is a conditional), [0607](0607-a-field-is-given-a-call-in-place.md) (a
variable a closure captured is given a call's value in place) and extends
[0846](0846-a-parameters-clone-is-read-by-its-name.md) (a clone of what a
`move` closure captured is read by its name).

Case ([0262](0262-when-rust-and-js-disagree.md)): N for each, which runs
the same; each is a form the program wrote, which MIR's structure lost.

## Context

React's `act` (`packages/react/src/ReactAct.rs`, react-in-rust) printed
five differences from upstream's `ReactAct.js`:

| Rust | Was | Upstream |
|---|---|---|
| `if !queue.borrow().is_empty() { flush.. } else { act_queue = None }` | `if (queue.length === 0) { actQueue = null; } else { flush.. }` | `if (queue.length !== 0) { flush.. } else { .. = null; }` |
| `if let Some(c) = continuation { .. } else { break }` | `if (continuation == null) { break; } ..` | `if (continuation !== null) { .. } else { break; }` |
| `if !DISABLE_LEGACY_MODE { x } else { false }` | `!disableLegacyMode && x` | `!disableLegacyMode ? x : false` |
| `result = callback();` in `catch_unwind`'s closure | `const value = callback(); result = value;` | `result = callback();` |
| `let prev = prev_act_queue.clone(); let queue = Rc::clone(&queue);` in a `move` closure | `const prevActQueue$1 = prevActQueue; const queue$1 = queue;` | `prevActQueue`, `queue` |

- **rustc turns `if !c`'s branches**: MIR building tests `c` and swaps the
  targets, so `switchInt(c)` goes to the `else` first.
- **A loop's early exit was any lone exit**: `early_return` made `if (c) { a
  } else { break; }` the guard `if (!c) { break; } a` whoever wrote it.
- **`conditional` folded any branch of `true` or `false`** into `&&` or
  `||`, the program's `&&` and its `if` alike, MIR making the same branches
  of each.
- **A place through a `Deref` kept its temporary** (0607), the JS place
  `a[i]` reached before the call; a closure's capture is `*(_1.0)`, whose
  JS is the variable itself.
- **A clone was read by its name only of a parameter** (0846).

## Decision

**An `if` the program wrote with an `else` (`Written::elses`) reads as it
wrote it:**

- **Its branches in their order**: `if (c) { b } else { a }` whose `a` is
  written in its `then` and `b` in its `else` is `if (!c) { a } else { b }`
  (`written_elses`).
- **Its `else` that leaves a loop stays an `else`**: `early_return` takes no
  `if (c) { a } else { e }` whose `a` is the `then` written and whose `e`
  none of it (`written_else`); not a `while`'s, whose desugared `if c { body
  } else { break }` has the whole loop for its `else`'s span, which stays
  the loop's test. 0237 still leaves out an `else` after a `then` that
  leaves; a `then` that goes on, with an `else` that leaves, is both
  branches.
- **Its value is a conditional**: `if c { x } else { false }` is `c ? x :
  false`, `if c { true } else { x }` `c ? true : x`; a value the program
  wrote with `&&` or `||` stays one, and `if c { true } else { false }` is
  `c`, as before (`written_branches`, `conditional`'s `written_if`).

**A variable a closure captured is given a call's value in place**, `result
= callback();`: a place JS reaches as a variable, with nothing to reach
first, is set once the value is made, as Rust sets it. One JS reaches
through an index keeps its temporary (0607).

**A clone of what a closure captured by value is read by its name**, as a
parameter's is (0846): the capture is a variable its maker never assigns
again from where it makes the closure (`mir_closure_fn` copies one it does,
`const captured = a`), so the clone, made in a call, is that value wherever
it's read, its closures' too. Not of one it borrowed, which its maker may
give another value once the borrow ends: `let b = a.clone(); move || *b` of
a borrowed `a` stays `const b = a`.

**`while c { ..; i += 1 }` stays a `while`** (not changed): the program
wrote a `while`, and `for i in a..b` is the `for`. A `for (; c; i++)` would
need a `for` of any test and step (the JS tree's `For` is a range's), and
would hold only where the body has no `continue`, which runs the step
before the test: two forms of one construct, picked by what the body holds.
Its `i = (i + 1) >>> 0` is a `usize`'s wrap where rust-js can't bound `i`,
`i < q.borrow().len()`, which `i < v.len()` bounds to `i++`.

## Why

- **It's the JS the program says**, and upstream's: `ReactAct.js` reads
  `queue.length !== 0` first, `else { break; }`, `!disableLegacyMode ? x :
  false`, `result = callback();` and `prevActQueue` in its callbacks.
- **It runs the same**: a test negated with its branches turned takes the
  branch it took; an `else` kept is the code that ran after the guard; `c ?
  x : false` of a `bool` is `c && x`; JS sets a variable after it makes the
  value; a clone that is the same JS value, of a variable nothing gives
  another, is that variable.
- **It's tested**: `test/corpus/branches_as_written.rs`, with native Rust,
  holds each, with a capture borrowed and given another value after its
  clone, and an index a call changes; `interpreter.rs`'s `if peek ==
  Comma { next } else { break }` and `generic_mut_ref_kept.rs`'s `if let
  .. else { return }` read as written.

## Alternatives

- **`if c { x } else { false }` as `c && x`**, 0364's: shorter, but two
  forms of one `if`, picked by a literal in a branch. Rejected.
- **Every clone of a `const` read by its name**: a `const` the function
  declares is fixed from where it's made too, but the program named the
  clone, `let gas = n;`'s choice of a second name (0846); left as it is.

## Consequences

- The guide's `is_adult`, `if let Some(age) = p.age { age >= 18 } else {
  false }`, is `p.age != null ? p.age >= 18 : false`; `p.age.is_some_and(|a|
  a >= 18)` is the `&&`.
- The playground's `ResultFrame` reads `onOutcome` in its handlers, where it
  read `told` and `silent`.
- `docs/readability.txt`'s `while (true)` share grows by one loop, 10.2 to
  10.3 in 10,000 lines: `branches_as_written.rs` writes a `loop` to hold
  its `if let .. else { break }`, and a `loop` is a `while (true)`.

## What holds it

- `test/corpus/branches_as_written.rs`, against native Rust;
  `test/lowering-branches.test.ts` ("branches from MIR read as a person
  writes them") and `test/lowering-written-branches.test.ts` ("a value's
  truth or else a bool is a bool part by part", whose `&&`, `||` and
  `match` keep the folds 0364 makes of what the program didn't write as
  an `if`).
- Mutations: `written-else-branches-as-mir-turned`,
  `written-else-branches-turned-untested`, `written-if-value-of-mir-branches`,
  `written-else-of-a-while-too` (`tidy/jumps/tails.ts`),
  `written-else-made-an-early-exit` (`tidy/jumps.ts`),
  `written-if-value-folded`, `written-chain-value-folded`
  (`tidy/branches.ts`),
  `variable-target-value-spilled` (`assign.ts`),
  `kept-captures-of-borrows-too` (`hints.ts`),
  `clone-alias-of-a-capture-kept` (`tidy/bindings.ts`).

## Amendment: one authority on a chain's branch order (#262)

`or_chain`'s `&&` branch in `tidy/branches.rs` picked its branches' order
by which the program wrote first, as `written`'s stage now does for every
`if` (above). The two answered one question, and the mutation that turned
the chain's branches (`mir-tidy-and-chain-turned`) survived, since the
`written` stage put them back. The chain now leaves the order to that
stage. `test/lowering-written-branches.test.ts`'s `&&` of loops and
`writer_callables.rs`, which reach it, read as before.

## Amendment: a returned value of turned branches

React's ReactEventPriorities writes `return a === 0 || a > b ? a : b;`;
its port, `if a == 0 || a > b { a } else { b }`, printed `a !== 0 &&
!(a > b) ? b : a`. MIR jumps to `b` where both of the `||`'s tests fail,
so the body is `if (a !== 0 && !(a > b)) { return b; } return a;`, a
jump alone each way, which `written_elses` leaves as it is, and
`returned_values` folded it in that order. **A returned value whose
branches MIR turned is the conditional as written**: where `written_order`
finds the `return` first in the `else` and the rest in the `then`, the
test is negated, `a === 0 || a > b`, and the branches swapped. Case N:
the negated test holds where the other doesn't, each branch where it was
taken. `written_branches` is `written_order`'s answer, either way round.
Held by `test/lowering-written-branches.test.ts`, "a returned value of an
if of an || keeps its test and its branches' order", `lane_bits.rs`
against native Rust, and the mutation
`written-turned-value-returned-as-mir-has-it` (`tidy/jumps/tails.ts`).

## Amendment: a written negation of a chain stays one

ReactFiberLane writes `.. && !(disableLegacyMode && root.tag ===
LegacyRoot)`; its port printed `(!DISABLE_LEGACY_MODE || root.tag !==
0)`, and `if !(a == 1 || b == 0)` printed `a !== 1 && b !== 0`: MIR
tests each part of a condition apart, so `!` of a chain is a chain of
each part's negation, De Morgan's. **A test's chain of `&&` and `||` is
`!( .. )` of the other operator where the program wrote that `!`**
(`Written::negations`, `logicals.rs`'s `written_negations`, in the
`written` stage before `turned`): where each of the chain's tests is
written inside the `!`'s operand, that operand is the innermost `&&` or
`||` written around them, and the chain is of the other operator; of a
run of a longer chain's parts too, which MIR joined into the chain around
the `!`, `.. && !disabled && tag !== 7` of `.. && !(disabled || tag ==
7)`. Only
of a test, an `if`'s, a `while`'s or a conditional's, whose truth is all
that's read. Case N: De Morgan's laws make the two the same test. A
chain the program wrote of negations, `a != 1 || b != 0`, has no `!` and
stays. `signed_counter.rs`'s `if !(s >= 1 && c >= 0)` reads so, where
it read `s < 1 || c < 0`, and so does an `assert!` of a chain, whose
expansion is `if !(cond)`: `array_vec_raw.rs`'s `!(index <= stack.len &&
stack.len < CAP)`. Held by `test/lowering-written-branches.test.ts`, "a written
negation of an && or an || stays one", run in JS, `lane_bits.rs`
against native Rust, and the mutations `written-negation-de-morganed`,
`written-negation-of-the-same-chain` and
`written-negation-of-a-whole-chain-only` (`tidy/logicals.ts`).

## Amendment: an `if` without an `else` keeps its body inside

React's getComponentNameFromFiber writes its `Throw` case `if (__DEV__) {
.. } return null;`; its port, `if __DEV__ { .. } return None;`, printed
`if (!__DEV__) { return null; } ..`, the body after the guard. MIR's
structure puts the short way out first, `if (!c) { exit } rest`, and
`written_elses` turned it back only of an `if` written with an `else`.
**An `if` the program wrote without an `else` (`Written::thens`) whose
`then` MIR made the statements after a guard is that `if` again**:
`if (!t) { exit } rest` is `if (t) { rest } exit` where `rest` is
written in the `then` and never goes on, as `exit` doesn't
(`jumps::written_thens`, the `chains` stage's `written thens`, after
`written elses`). A `let … else`'s guard, which isn't an `if`, and an
`if` MIR turned whose exit is its `then`, `if (!href.startsWith(..)) {
return next; } return ext;`, stay as they were. Case N: each way, the
test that holds runs the branch it ran; `rest` never goes on, so what
follows it is the exit either way.

The `if` keeps the guard's own exit too: the port's `?` inside it,
`let r = fiber.return.borrow().clone()?;`, returns `null` there, as
upstream's `if (fiber.return === null) { return null; }` does. In the
corpus, `loop_break_choice.rs`'s `if c > 3 || id(false) { break .. }
return -1;` reads so, where it read `if (c <= 3 && !id(false)) { return
-1; }`, and a loop's `if c > id(limit) || note(c, stop) { break .. }`
too, where it read a `continue` of the negated test.

Held by `test/lowering-written-branches.test.ts`, "an if the program
wrote before a return keeps its body inside", run in JS, the unit test
`written_thens_turn_only_a_rest_that_leaves` (`tidy.rs`), of a rest that
goes on, `loop_break_choice.rs` against native Rust, and the mutation
`written-thens-as-mir-has-them` (`tidy/jumps/tails.ts`).

## Amendment: a `let … else` before a written `return` is a guard first

React's getCurrentFiberStackInDev writes `if (__DEV__) { if (current ===
null) { return ''; } // Safe because .. return
getOwnerStackByFiberInDev(current); } return '';`. Its port, `let
Some(current) = CURRENT.with_borrow(..) else { return String::new(); };
// Safe .. return get_owner_stack_by_fiber_in_dev(&current);`, printed `if
(current != null) { // Safe .. return ..; }` with no guard at all: MIR
joins the guard's `return` and the rest's at the one return block, so the
`if` is `if (c) { s } e`, not the `if (c) { s } else { e }`
`let_else_first` turns; and `shared_tails` (ADR 0564) took `e`, `return
"";`, as a copy of the function's tail it's alike.

**A `let … else`'s `else` is a guard first wherever MIR joins it**
(`Written::let_elses`, where each `else` is written): `let_else_first`
turns `if (c) { s } e`, `s` never going on and `e` the rest, which
leaves, into `if (!c) { e } s` where each of `e`'s statements is written
in one `let … else`'s `else` and none of `s`'s; and `shared_tails` keeps
a branch's copy written in one, the guard's exit, not the tail. Only of
a `let … else` the program wrote: an exit written before the rest is no
guard by itself, a `?`'s `if (r.TAG === "Err") { return .. } const x =
r._0;` written after its rest's `let x`, already first. Case N: the
guard's test negated holds where the other doesn't, each branch where it
was taken, and the rest never goes on, so nothing follows it either way.

Held by `test/lowering-early-exits.test.ts`, "a let else before a return
is a guard first", run in JS, `let_else_guard_returns.rs` against native
Rust, and the mutations `let-else-joined-return-not-guard`,
`let-else-joined-return-of-any-exit` (`tidy/jumps/tried.ts`) and
`shared-tails-take-a-let-elses-exit` (`tidy/branches.ts`).

## Amendment: a `!` of a chain the program didn't write is De Morgan's

React's ReactFiberThenable checks `thenable.status !== 'fulfilled' &&
thenable.status !== 'rejected'`. Its port's `!matches!(status,
Some("fulfilled" | "rejected"))` was `return !(status === "fulfilled" ||
status === "rejected");`: MIR's value of the `matches!`, a chain of its
pattern's alternatives, read in place under the `!` the program wrote of
the macro.

The rule above keeps a `!( .. )` the program wrote of an `&&` or an `||`.
`!matches!(..)` is no such `!`: the chain is MIR's of an or-pattern, which
the program wrote as alternatives, `"fulfilled" | "rejected"`, not as a
`||`. **A `!` of a chain the program didn't write as one is each part's
negated test, joined by the other operator, De Morgan's**:

```js
return status !== "fulfilled" && status !== "rejected";
```

- **Only where each part negates exactly**: an equality's test, `===`
  as `!==`, or a `!`'s, its operand. A range's, `!matches!(x, 1.0..=3.0)`,
  stays `!(x >= 1 && x <= 3)`, as `x < 1` isn't `!(x >= 1)` of `NaN`.
- **Of a value too**, not only a test, as `!` and each part are `bool`s
  alike: what `&&` and `||` give of `bool`s is a `bool`.
- **A written one stays**, `return !(a === 1 || b === 0);` of `!(a == 1
  || b == 0)`, by the same `Written::negations` (`written_negation`).

Case N ([0262](0262-when-rust-and-js-disagree.md)): De Morgan's laws make
the two the same value. Tested by `test/lowering-written-branches.test.ts`'s
"a negated matches! of alternatives", run; its mutations are
`unwritten-negation-kept`, `written-negation-de-morganed-in-a-value` and
`unwritten-negation-of-inexact-tests` (`tidy/logicals.ts`).

## Amendment: an `else` after a branch that returns stays where statements follow

Superseded by the user's decision of 2026-10-28 ([ADR 0237](0237-no-else-after-leaving.md),
amended): every `else` the program wrote stays, statements after the `if`
or not.

The user's decision (2026-10-27). React's insertOrAppendPlacementNode
writes `if (isHost) { ..; return; } else if (tag === HostPortal) { //
.. return; }` and goes on after it; its port, the same `if .. else if`,
printed two `if`s, the `else` gone. MIR's structure makes a guard of
the `then` that returns; `written_elses` made the `else` again only of
every statement after the guard, so the program's statements after the
`if` kept it a guard; and where it did make one, preparation's
`unelsed` (ADR 0237) took it away again.

**An `else` the program wrote after a branch that leaves stays an
`else` where statements follow the `if`:**

- `written_elses` makes the `else` of the statements after the guard up
  to the last written in it, where a statement the program wrote after
  the whole `if` follows them, and every one after them is written there
  or nowhere. A `return` MIR made of the function's value is where its
  value is written, not where the function ends. Where nothing written
  follows, the `else` is all of what's after the guard, as before.
- Its test says so, `Known::Else`, and `unelsed` keeps that `else`
  where a statement follows the `if` in its block. A chain's `else
  if`s inside it are links, kept as before.
- **Where nothing follows**, the `if` the function ends with, ADR 0237
  still holds: `if (a) { return 1; } if (b) { return 2; } return 3;` of
  `if a { return 1; } else if b { return 2; } else { 3 }`, as ESLint's
  `no-else-return` writes it. `if`s the program wrote apart stay apart.
- A loop's `if x == 0 { continue; } else if .. { break; }` isn't
  covered yet: MIR's jumps there are the loop's, which `written_elses`
  doesn't see as a guard's exit.

Case N ([0262](0262-when-rust-and-js-disagree.md)): after a branch that
leaves, what follows runs only where it didn't, `else` or not.

```js
if (isHost) {
  log.push(tag);
  return;
} else if (tag === 4) {
  // A portal's children are placed on their own.
  return;
}
log.push(0);
```

So does an `else` that leaves after a `then` that goes on, as 0879
already had it, where a statement follows: React's formatOwnerStack,
`if let Some(idx) = idx { stack = &stack[..idx]; } else { return
String::new(); } stack.to_string()`, is upstream's `if (idx !== -1) {
stack = stack.slice(0, idx); } else { return ''; } return stack;`, where
it read `stack = ..; return stack; } return "";`.

ADR 0958's `switch` of a chain of `js::strict_equals` tests reads the
chain so too: an `else` written after the chain's `return`s, statements
after it, is the `default`, and what follows is after the `switch`,
`default: if (s !== c) { return 3; } log.push(4); } return 4;`; `if`s
written apart stay `if`s.

Held by `test/lowering-written-branches.test.ts`'s "an else after a
branch that returns stays where statements follow", run in JS, and "an
else-if chain keeps its else past an arm that returns", whose `guarded`
chain now stays one; `test/lowering-string-places.test.ts` and
`test/lowering-shadows.test.ts`'s formatOwnerStack,
`test/lowering-strict-switches.test.ts`; and the mutations
`written-else-only-of-the-whole-rest`, `written-else-unmarked`,
`written-else-return-at-its-span` (`tidy/jumps/tails.ts`),
`written-else-unelsed-though-followed` and
`written-else-kept-at-the-end-too` (`prepare.ts`).

## Amendment: a function's last `match` of arms that give `()` is its arms

The user's decision (2026-10-27). React's commitLayoutEffectOnFiber
writes `if (current === null) { if (shouldProfile) { .. } else { try {
.. } catch (error) { .. } } } else { .. }`, no `return`. Its port, `match
current { None => { if .. { .. } else if let Err(error) =
catch_unwind(..) && .. { .. } } Some(prev) => .. }` at the function's
end, printed `if (current == null) { if (..) { ..; return; } try { ..;
return; } catch (error) { ..; return; } } ..`: MIR ends each way with
the function's `return`, which `tail_returns` dropped only where an
arm's own statements ended with it, not where they ended in an `if`
each of whose branches did.

**`tail_returns` takes what ends the function whole**: an `if` whose
`then` ends with a `return;` the program didn't write each way, through
its branches, is `if (c) { .. } else { rest }`, and the branches of the
`if` that ends the function are tidied so too, each `return;` MIR made
at their ends gone. A `return` the program wrote stays (`written`).
Case N ([0262](0262-when-rust-and-js-disagree.md)): a `return;` where
the function ends anyway does nothing, and the `else` runs where the
`then` didn't, as the `return` made it.

```js
if (current == null) {
  if (shouldProfile()) {
    didMount();
  } else {
    try {
      didMount();
    } catch (error) {
      capture(error);
    }
  }
} else {
  didUpdate(current);
}
```

Held by `test/lowering-written-branches.test.ts`'s "a function's last
match of arms that give () is its arms, no returns", run in JS; and the
mutations `tail-returns-branch-else-after` and
`tail-returns-of-branches-only-at-the-top` (`tidy/jumps.ts`).

## Amendment: `let … else`s that each `break` a loop are each its guard (2026-10-27)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same statements
run on each path.

React's findCurrentFiberUsingSlowPath (c11 of the port's gaps):

```js
while (true) {
  const parentA = a.return;
  if (parentA === null) {
    // We're at the root.
    break;
  }
  const parentB = parentA.alternate;
  if (parentB === null) {
    ..
    // If there's no parent, we're at the root.
    break;
  }
  ..
}
```

Its port's two `let … else`s each `break`, and MIR joins both `break`s
into one after them: `if (parentA != null) { ..; if (parentB != null) {
..; continue; } .. } break;`, each guard nested in the one before, and
their comments printed before the loop, where no statement of theirs was.

- **`if (c) { s } j` of a lone jump `j` that isn't written in the `else` of
  the `let … else` whose test `c` is**, its `else` written right after the
  test and before `s`: `if (!c) { j } s`, and `j` after `s` where `s` goes
  on to it.
- **`if (c) { s } else { e } j` of an `s` that never goes on and an `e`
  written in a `let … else`'s `else`** that goes on to `j`: `if (!c) { e j }
  s`.
- **Each such guard's test says it's one (`Known::Guard`)**, so
  preparation's loop of a test (ADR 0860), which takes `while (true) {
  const x = ..; if (x == null) { break; } .. }` for `while ((x = ..) !=
  null)`, leaves it as written: a `while let` or a `for`'s steps, which
  MIR makes alike, are still that loop.

Held by `test/lowering-early-exits.test.ts`, "let elses that each break a
loop are each a guard first", run; the mutations
`let-else-joined-break-nested`, `let-else-joined-break-left-out`
(`mir/tidy/jumps/tried.ts`) and `let-else-guard-a-loop-head`
(`prepare.ts`).

The port's own findCurrentFiberUsingSlowPath doesn't take this shape yet:
MIR places the code after its loop inside it, `if (a.tag !== HostRoot) {
break; } .. return ..;` before the loop's end, and the `break`s go there,
which is still to do.

## Amendment: `if`s written without an `else` that each `break` a loop end with it (2026-10-27)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same statements
run on each path: the jump after the chain was reached only from its
`then`s.

React's findCurrentFiberUsingSlowPath walks a child list, `while (child) {
if (child === a) { ..; break; } if (child === b) { ..; break; } child =
child.sibling; }` (c12 of the port's gaps). MIR joins the two `break`s
after the `if`s, and its port printed `if (child === a) { .. } else if
(child === b) { .. } else { child = child.sibling; continue; } break;`.

**A chain of `if`s the program wrote without an `else`
(`Written::thens`), followed by the jump MIR joined their ends into, its
last `else` the rest, which never goes on, is each `if` ending with that
jump and the rest after them** (`tails::written_jumps_of_ifs`), in tidy's
`written` stage: `if (child === a) { ..; break; } if (child === b) { ..;
break; } child = child.sibling;`. An `else` the program wrote stays.

Held by `test/lowering-early-exits.test.ts`, "ifs that each break a loop
are each written with their break", run, beside an `else if` chain the
program wrote; the mutations `if-jumps-chained-past-the-first` and
`if-jumps-of-a-written-else` (`mir/tidy/jumps/tails.ts`).

## Amendment: a unit function's early `return`s MIR joined to its end are each a guard (2026-10-27)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same statements
run on each path: falling off a function's end is its `return;`.

React's recordLegacyContextWarning, `const strictRoot =
findStrictRoot(fiber); if (strictRoot === null) { console.error(..);
return; } if (didWarn.has(fiber.type)) { return; } if (..) { .. }` (c20
of the port's gaps). Where a `let … else` takes apart a local an `Rc`
drop follows, MIR joins each `return` to the function's end, and its port
printed `if (strictRoot == null) { console.error(..); } else { if
(!didWarn.has(..)) { .. } }`: no `return` to make a guard of, as the
`returned` pass had taken the one the function ends with away.

**Tidy's `let else first` pass sees a unit function that goes on to its
end as ending in `return;`** (`stages.rs`), then takes it away again: a
`let … else`'s `else`, written first, `if (c) { e } else { s }`, is its
guard, `if (c) { e return; } s` (`tried::let_else_first`); and each `if
t { return; }` the program wrote without an `else` (`Written::thens`, the
`return` written in it) before the rest, `if (!t) { rest }`, is `if (t) {
return; } rest` (`tails::written_exits`), in turn. An `if` whose `then`
the program wrote with the rest in it stays. The playground's `new_file`'s
`if path.is_empty() { return; }` is its guard too, as written.

Held by `test/lowering-early-exits.test.ts`, "a unit function's early
returns joined to its end are each a guard first", run on each path; the
mutations `let-else-first-at-the-end-nested`,
`let-else-first-at-the-end-unended` (`mir/tidy/jumps/tried.ts`),
`written-exits-at-the-end-nested`, `written-exits-of-a-then-written`
(`mir/tidy/jumps/tails.ts`) and `let-else-first-unit-end-unseen`
(`mir/tidy/stages.ts`).

## Amendment: a written conditional of `bool`s with a literal branch is `&&` or `||` (2026-10-28)

The user's decision (2026-10-28), which supersedes this decision's "its
value is a conditional" for these two forms, and the alternative it
rejected. Case ([0262](0262-when-rust-and-js-disagree.md)): N, below.

A value match at a function's end reads as a written `if`'s since the
user's "a match's last arm is its else" (ADR 0237, amended), so
`match s.find('b') { Some(at) => at < 3, None => false }` printed
`return at != null ? at < 3 : false;` where it had printed the `&&`.
React's isFragmentInstanceHostBoundary writes `supportsSingletons ? tag
=== HostSingleton : false`, but most of the port's values of this shape
are upstream's `&&` and `||`.

**An `if` or `match` the program wrote with an `else`, of `bool`s, one
branch a literal, is the operator it means:**

- `c ? b : false` is `c && b`, and `c ? true : b` is `c || b`,
  `logicals.rs`'s `conditional` of `written_if`, which moved there from
  `branches.rs` (a file's cap): `return at != null &&
  at < 3;`, `const chosen = fast || slow();`, `return
  !DISABLE_LEGACY_MODE && legacy;`.
- **Only where `c` and `b` are `bool`s**, as `is_boolean` says of each
  (a Rust `bool`'s variable or temporary, a comparison, a `!`, a binding's
  `bool`): `dev && globalThis.bigIt(n)`, `dev && $index(flags, 1)`. A
  test of a value's truth, `if js::truthy(v) { n > 2 } else { false }`,
  stays `v ? n > 2 : false`, as `v && ..` would give `0` of `v = 0`; and
  so does a branch that isn't a `bool`, an `Option<bool>`'s `if dev { n }
  else { Some(false) }`, `dev ? n : false`.
- The other two forms, `c ? false : b` and `c ? b : true`, stay as
  written: their `&&` and `||` would negate a test the program didn't.
  What MIR made, of an `&&`, `||` or `matches!` the program wrote, folds
  every form, as before.

Case N: of a `bool` `c`, `c && b` is `b` where `c` is `true` and `false`
where it isn't, as the conditional is, and `c || b` `true` or `b`; `b`
is read only where the conditional reads it.

`docs/readability.txt` doesn't move. The guide's `is_adult` reads `p.age
!= null && p.age >= 18`; the playground's `ResultFrame` `fromFrame =
frame$1 != null && Object.is(..)`.

Held by `test/lowering-bool-ternaries.test.ts`, "a written if or match
of a bool literal branch is && or ||", run in JS, of each kind kept;
`branches_as_written.rs`, `option_some_mut.rs` and
`string_places_kept.rs` against native Rust; and the mutations
`written-bool-ternary-kept`, `written-bool-or-ternary-kept`,
`written-bool-ternary-of-any-branch` and
`written-bool-ternary-of-any-test` (`tidy/logicals.ts`).

## Amendment: a written `if` whose rest goes on to the function's end is that `if` (2026-10-28, wl6)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same statements
run on each path: what goes on to a function's end returns there.

React's throwAndUnwindWorkLoop's `Ok` arm, `if (didFatal) {
panicOnRootError(..); return; }`, then the code after the `try`: its
port's `Ok(did_fatal) => { if did_fatal { panic_on_root_error(..);
return; } }` printed `if (!didFatal) { .. } else { panicOnRootError(..);
}` after the `try`. MIR joins the written `return` to the function's
end, so the `if`'s rest, `panicOnRootError(..)`, goes on there, and
`written_thens` turned back only a rest that leaves.

**A way through the body that goes on to its end, or to the `return;`
it ends with, leaves as that `return;` does** (`returns_at_ends`, in the
`written thens` pass): each is given a `return;` of MIR's, the rest
leaves, and `written_thens` makes the `if` as written, `if (didFatal) {
panicOnRootError(..); return; } ..`, its `return;` the one the program
wrote in the `then`, which `tail returns` keeps; one still at a way's
end, MIR's, `tail returns` takes away, as before.

Upstream writes that `if` inside the `try`; the port's `if` is in the
`Ok` arm, outside what `catch_unwind` catches, so a throw of
`panicOnRootError` isn't the `catch`'s, and the JS keeps it after the
`try`. A port that wants upstream's shape writes the `if` in the closure,
its `return` a flag out of it (ADR 0660, amended).

Held by `test/corpus/written_tails_apart.rs`, against native Rust;
mutations `written-thens-not-to-the-end` (`tidy/stages.ts`) and
`written-thens-end-return-unwritten` (`tidy/jumps/tails.ts`).

`docs/readability.txt`'s share of `return;` grows, 9.8 to 9.9 in 10,000
lines, by that `if`'s: the `return;` the program wrote in it, which MIR
joined to the function's end.

## Amendment: a `?`'s way out is a guard first, as a `let … else`'s (2026-10-28, q1)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the same statements
run on each path; the guard's test negated holds where the other didn't.

React's findHostInstance writes `const hostFiber =
findCurrentHostFiber(fiber); if (hostFiber === null) { return null; }
return getPublicInstance(hostFiber.stateNode);`. Its port's `let
Some(host_fiber) = host_fiber else { return None; };` printed that, but
clippy's `question_mark` makes it `let host_fiber = host_fiber?;`, which
printed `if (hostFiber != null) { return getPublicInstance(..); } return
null;`: MIR joins the `?`'s `return` with the function's, after the
`Rc`'s drop, and nothing said the `?` is a guard.

**A `?` is a guard's exit where it's written, as a `let … else`'s
`else` is** (`Written::let_elses`, of each `?`'s desugared `match`):
`let_else_first` makes `if (c) { s } e` the guard first, `if (!c) { e }
s`, where `e`, its `return`, is written at the `?`; a `return`'s place is
its value's, as each way of a `?` returns at the `?` itself. The
guide's `title_of` reads `if (value == null) { return; } return ..;` so
too, and a `let … else { return match .. }` whose value is written in its
`else` is its guard first, `if (tag !== HostText) { .. }`.

Held by `test/lowering-written-elses.test.ts`, "a ? of an Option<Rc> is
its guard first", run in JS; mutations `question-exit-not-a-guard`
(`hints.ts`) and `let-else-exit-at-its-statement`
(`tidy/jumps/tried.ts`).
