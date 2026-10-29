# 0237. An `if` whose branch leaves has no `else`

Status: Accepted. Amended by [0879](0879-an-if-and-a-closures-captures-read-as-written.md): an `else` the program wrote whose `then` goes on, `if let Some(c) = c { .. } else { break }`, is both its branches, not an early exit; and an `else` the program wrote after a branch that leaves stays where statements follow the `if` (the user's decision, 2026-10-27), so this rule holds of an `if` that ends its block, or one MIR made. Amended again (the user's decision, 2026-10-28): every `else` the program wrote stays, statements after the `if` or not, and an `else if` chain is one; this rule holds of an `if` the program wrote without an `else`, MIR's guard of it. Amended again (the user's decision, 2026-10-28): a `match` of two arms is an `if` with both its branches, its last arm the `else`.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

Rust's `if c { a } else { b }` is one expression, and rust-js wrote both
branches alike, so a function's last `if` returned in each:

```js
if (isValidElement(child) && child.type.mdxName === "inlineCode") {
  return cloneElement(child, { isLink: true });
} else {
  return child;
}
```

react.dev's own code writes the `return` after the `if`, 88 times to the
`else`'s 12 in its source, as its Link does: once the branch has left,
what's after it runs only when it didn't. ESLint's `no-else-return` says so.

## Decision

**An `if` whose branch always leaves, by a `return` or a `throw` last, or
an `if` both of whose branches do, has no `else`: what was in it follows
the `if`.**

```js
if (isValidElement(child) && child.type.mdxName === "inlineCode") {
  return cloneElement(child, { isLink: true });
}
return child;
```

- **An `else if` chain is each `if` after the last**, while each leaves.
- **A branch that doesn't leave keeps its `else`**: `if (n > 0) { total
  += n; } else { total -= n; }`.
- **Its locals stay apart**, now in the block the `if` is in: they're
  named once it's there, as the block's (ADR 0357).
- **It's how the JS is written, not what rust-js lowers**: preparation
  makes it, before its locals are named, and the printer prints what it's
  given. (Amended: the printer wrote it so, and preparation's scopes
  modelled what the printer would do to name the locals beside it.)

## Why

- **It's the JS a person writes.**
- **It runs the same**: after a branch that leaves, nothing that follows
  runs, and nothing else did; inside a `try`, a `return` still runs its
  `finally`.
- **It's tested**: a compiler test writes and runs an `else if` chain that
  returns, a branch that leaves by an inner `if`, and one that doesn't;
  the corpus runs every changed program beside native Rust.

## Since

- **An `else` of nothing is no `else`**: one of only comments in Rust,
  which rust-js doesn't carry, as react.dev's _app has, is left out.
- **A branch that ends by a `break` or a `continue` has no `else`
  either**, nor one whose last statement is a `while (true)` nothing
  breaks: `if (item === undefined) { break; } const cell = item;`, as a
  person writes a loop. Whether a statement goes on to what follows is
  `js::ends`' answer, the one MIR's tidy pass asks too; preparation had
  its own, which knew only `return` and `throw` (the second architecture
  review).
- **An `else if` of an arm that goes on stays one** (ADR 0683): `if (a) {
  x = 1; } else if (b) { return; } else { .. }` keeps its chain, as
  ESLint's `no-else-return` allows; an `else` goes only where each arm
  before it leaves too.

## Amendment: which `else` is a link is the walk's to say

`unelsed` held the `else` lists of an `else if` chain as a set of their
addresses, filled as the walk went from the outside in, and asked of each
list as it came: right only while the walk visits a parent before its
children and leaves each list where it is till then (a review,
2026-10-24). The walk now says where each list is, `js::Slot`, an `if`'s
`else` or another block (`statement_lists_in`,
`each_item_slotted_block_mut`), and a link is an `else` whose one
statement is an `if`. The JS is the same, byte for byte. Held by the
mutations `else-if-links-unheld` and `else-slot-unsaid`.

## Amendment: every `else` the program wrote stays (2026-10-28)

The user's decision (2026-10-28), superseding 2026-10-27's (ADR 0879,
amended), which kept a written `else` only where statements followed
the `if`. React's BeginWork writes both arms, `if (current === null) {
..; return x; } else { .. }`, in updateActivityComponent,
updateSuspenseComponent, updateHostRoot and the dehydrated components;
other files write `no-else-return`'s style. The Rust says which, as
upstream does file by file:

- **An `else` the program wrote stays an `else`**, at the end of its
  block too: `if (key === 1) { return 10; } else { return key + 1; }`,
  `if (payload._status === "Resolved") { return payload._result; } else
  { throw payload._result; }`.
- **An `else if` chain is one**, where each arm leaves and where it ends
  a branch: `if (key === 2) { return 0; } else if (key === 3) { return 1;
  } else { .. }`.
- **What the program wrote after the `if` follows it**, out of the
  `else` MIR's structure put it in: `} else if (tags.length === 0) {
  return 7; } return 9;`.
- **An `if` the program wrote without an `else`** has none still, `if (c)
  { return; } more`; nor a value's `if` of a lone jump MIR made each way,
  `return c ? a : b`'s; nor an `else` of only the `return;` MIR made of a
  `None` or a `()`, which, at the function's end, is nothing, as falling
  off it gives `undefined` too.

`written_elses` (`tidy/jumps/tails.rs`) marks the test of each `if` it
makes of a guard and of each it finds with both branches the program
wrote (`written_else`), `Known::Else`, and preparation's `unelsed` keeps
each marked `else`. An `else` it makes holds the chain's next link,
which it reads again, a value's too. `tail_returns` drops a `return
undefined;` MIR made at the end as it drops a `return;`.

Case N of [ADR 0262](0262-when-rust-and-js-disagree.md): JS's `else`
after a branch that leaves runs as the statements after it would; which
reads is the program's.

Tested by `test/lowering-written-branches.test.ts`'s "an else after a
branch that returns stays, as written" and the corpus
(`returns_in_held_branches`, `option_bool_tested`, `unit_given_to_js`,
`restarting_iterators`); mutations `written-else-unelsed`,
`guarded-else-unmarked`, `both-branches-else-unmarked`,
`written-else-keeps-what-follows`, `else-if-link-not-a-link`,
`plain-jumps-marked-written` and `lone-jumps-given-an-else`.

## Amendment: a written `else { break; }` of a loop's chain stays

React's getOwnerStackByComponentInfoInDev (ReactFiberComponentStack.js)
walks its owners with a chain whose last arm is a `break` the program
wrote:

```rust
while let Some(current) = owner {
    if .. {
        ..
    } else if current.debug_stack.is_some() {
        ..
    } else {
        break;
    }
}
```

The chain printed `} else { if (owner.debugStack != null) { ..; continue;
} break; }`, and a `for`'s of an arm that returns before it `} else { if
(!(..)) { break; } .. }`. The written `else { break; }` reaches tidy as
a jump MIR made, of no span, or, once `early breaks` has made it the
guard `if (!b) { break; }`, of its test's span: `written_elses` found no
statement written in either branch and left the guard.

- **At a loop body's end, `if (b) { ..; continue; } break;` of an `if`
  the program wrote with an `else`, both jumps MIR's, is `if (b) { .. }
  else { break; }`** (`tail_continues`), the `else` marked written.
- **A guard whose exit is a lone jump MIR made, or put where the test is,
  is the written `else`'s** where the rest is written in the other
  branch (`written_elses`); the jump loses the test's span, so `let else
  first` doesn't take it for a `let … else`'s, written first. Not of a
  `while let`'s own test, `if (next == null) { break; } ..`, whose `else`
  is the whole loop: taken for one, it left React's
  propagateContextChanges `break; break;`, which a debug build's contract
  refused (mutation `written-else-of-a-while-test`).

Each is the same jump either way. Held by `test/corpus/else_break_ends_loop.rs`,
against native Rust; mutations `written-else-break-continued`,
`written-else-of-a-lone-jump-guarded` and `written-else-jump-span-kept`.

## Amendment: a `match`'s last arm is its `else` (the user's decision, 2026-10-28)

React's updateActivityComponent, updateSuspenseComponent and
updateHostRoot write `if (current === null) { ..; return ..; } else { ..
}`. Their ports' `match current { None => { ..; return ..; } Some(c) => {
.. } }` printed `if (current == null) { ..; return ..; } ..`: an `else`
the program wrote stays (the amendment above), and a `match` wrote none.

**A `match` of two arms is an `if` the program wrote with an `else`, its
first arm the `then`, its last the `else`**, as `if let` and `if`/`else`
are: a statement `match` at a function's end prints `if (c) { .. } else {
.. }`, and a value's `return c ? a : b`, as a written `if`/`else`'s value
does ([0879](0879-an-if-and-a-closures-captures-read-as-written.md)):

```js
export function tailMatch(current, wip) {
  if (current == null) {
    if (wip.lanes === 1) {
      return 0;
    }
    return wip.lanes;
  } else {
    wip.lanes = current.lanes;
    return current.lanes;
  }
}

export function valueMatch(n) {
  return n != null ? (n * 2) >>> 0 : 1;
}
```

- **Where it's one** (`hints.rs`'s `Written::elses`): two arms, neither
  guarded nor inside an arm's guard, neither a jump alone, `None =>
  break`, which stays a guard's way out as `let … else` writes it. Its
  test is where MIR tests it, from `match` to the scrutinee's end, or to
  the first arm's pattern's, where a string's `s === "a"` is.
- **A `match` of constants is a `switch` first**
  ([0629](0629-a-match-of-constants-is-a-switch.md)): `returned ifs` and
  `returned values` leave one alone until `switches` has made what it can,
  `switch (tag) { case HostHoistable: case HostComponent: return 2;
  default: return 3; }`; `unswitched values`, after it, folds one it
  didn't, two arms of an enum's four.
- **A written negation keeps the mark**: tidy's `negations`, which makes
  each `!x` again, kept none of the test's facts, and a written `else`
  whose test is a negation, `if !flag { ..; return 1; } else { .. }`'s,
  lost its `Known::Else` before preparation's `unelsed` read it, a bw2
  leftover.
- **An arrow called in place is no small value** (`branches.rs`'s
  `small`): a `with`'s closure that throws, `((e) => { throw e; })(e)`,
  stays a branch of its own, `if (done) { return 1; } throw exception;`,
  not a conditional of it.

A port that wants no `else`, upstream's `if (..) { return ..; } ..`,
writes `let Some(c) = current else { ..; return ..; };` or an `if let`
with no `else`; getIODescription's port writes `if let Kind::String(name)
= .. { return name.to_string(); } String::new()` for upstream's `if
(typeof name === 'string') { return name; } return '';`.

Case N ([0262](0262-when-rust-and-js-disagree.md)): an `else` after a
branch that leaves runs as the statements after it would, and a
conditional gives the value the branch returned.

Held by `test/corpus/match_last_arm_else.rs`, against native Rust, and
the corpus's snapshots; mutations `match-arms-no-else`,
`match-arm-jump-alone-an-else` (`mir/hints.ts`), `negation-else-dropped`
(`mir/tidy.ts`), `switch-match-returned-first`, `unswitched-values-off`
(`tidy/stages.ts`) and `arrow-called-in-place-small` (`tidy/branches.ts`).

## Amendment: an `&&` test keeps its `if`'s written `else` (2026-10-28, rg3)

React's throwException writes `if (getIsHydrating() && (disableLegacyMode
|| sourceFiber.mode & ConcurrentMode)) { .. return ..; } else { //
Otherwise, fall through to the error path. }`. Its port printed no `else`,
the comment after the function's last statement, and so did every written
`else` of an `if` whose test is an `&&`, `if (h && (l || ..)) { ..;
return; } else { .. }`: preparation makes an `if`'s test again as a test
(`tested`) and an `&&`'s of its parts (`typeofs::implied`), which keeps
none of the test's facts, so `unelsed` no longer saw `Known::Else`.

**What a test says of its `if`, `Known::Else` and `Known::Guard`, is kept
whatever preparation makes of the test** (`prepare.rs`'s `block`). Case
N: the same branches either way. Held by
`test/lowering-comments-ends.test.ts`'s "an else of only comments after a
branch that leaves stays", run in JS, and `match_last_arm_else`'s
`anded`, against native Rust; mutation `if-facts-lost-to-its-test`.

## Amendment: a check written apart, and a bit test made again, keep the `if`s written (2026-10-28, me2)

Two regressions of "a match's last arm is its `else`" (above), found in
the React port. Case ([0262](0262-when-rust-and-js-disagree.md)) N for
each: the same statements run on each path.

- **React's markRef**, `} else { if (typeof ref !== 'function' && ..) {
  throw .. } if (current === null || ..) { .. } }`: its port's last arm,
  now the `else`, printed `} else if (..) { throw .. } else if (..) { ..
  }`. Tidy's `chained_checks` makes `else { if (t) { throw e; } s }` the
  chain `else if (!t) { s } else { throw e; }`, which is what MIR makes of
  `else if c { s } else { panic!() }`; it took the program's own `if`,
  written without an `else`, too. **A check the program wrote as an `if`
  of its own (`Written::thens`) stays one.**
- **React's updateHostRoot**, `if (workInProgress.flags &
  ForceClientRender) { return ..; } else if (nextChildren !==
  prevChildren) { .. }`: its port's `(flags & FORCE_CLIENT_RENDER) != 0`,
  which `bit_tests` makes the truth test `flags & 4`, printed `} if
  (next !== prev) {`, the rebuilt test without `Known::Else`. **What a
  test says of its `if` is one set, `Facts::of_statement` (`Known::Else`,
  `Known::Guard`), kept by each pass that makes a test again of the same
  truth**: `bit_tests`, tidy's `negations` and preparation's `block`,
  which each named the facts on their own before.

Held by `test/lowering-written-elses.test.ts`, "a written if that throws
in a match's last arm stays an if of its own" and "a bit test made again
keeps its if's written else", run in JS; mutations
`chained-checks-of-written-ifs` (`tidy/branches.ts`),
`bit-test-drops-its-else` (`tidy/logicals.ts`), and
`negation-else-dropped`, `if-facts-lost-to-its-test` on the shared set.

## Amendment: `tail_continues` no longer makes the written `else { break; }` (2026-10-29)

The first rule of the amendment above, `tail_continues`' `if (b) { ..;
continue; } break;` into `if (b) { .. } else { break; }`, changed no
program's JS by the time the nightly ran its mutation,
`written-else-break-continued`: `written_elses` and the chains' passes
make the same `else` first, and `else_break_ends_loop.rs`'s snapshot, with
the whole corpus's and the lowering, snapshot, JSX, port, compiler and
tidy-stage suites, was unchanged without it. It goes with its mutation.
