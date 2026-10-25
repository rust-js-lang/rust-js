# 0684. A `continue` the program wrote is where it wrote it, with its comment

Status: Accepted. Extends [0444](0444-a-comment-is-where-its-code-is.md),
whose comments are before the statement whose code follows them, and
[0575](0575-an-if-keeps-its-else-and-a-tail-if-is-a-conditional.md)'s
`Written::continues`. Amended by
[0730](0730-a-loops-early-break-is-in-the-if-it-was-written-in.md): a loop
body's last `if` is an early `continue` only where the program wrote one.
Amended below: a loop's `break` the program wrote is where it wrote it too
(#298).

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
