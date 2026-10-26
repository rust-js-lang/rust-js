# 0693. A guard's scope is one `finally`, its `if`s, loops and `return`s in it

Status: Accepted; amended (around a `catch`, one `try`, `catch` and
`finally`; a function or a constant returned last, after it; the
function's return place given from inside it). Amends [0437](0437-a-scopes-drops-are-one-finally.md),
whose run of statements a drop ends was flat: an `if` or a loop in it, or
a `return` from inside it, ended the run.
Amended by [0705](0705-a-guards-scope-reads-as-any-body.md): what each
statement's `try` held apart is read in place, and a guard's field of a
variable is that variable.

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same drops in the
same order on every path; only how the JS reads changes.

## Context

React's `describeNativeComponentFrame` restores what it changed in one
`finally`, however it's left:

```js
reentry = true;
..
try {
  ..
  for (; s >= 1 && c >= 0; s--, c--) {
    if (..) {
      ..
      return frame;
    }
  }
} finally {
  reentry = false;
  ReactSharedInternals.H = previousDispatcher;
  reenableLogs();
  Error.prepareStackTrace = previousPrepareStackTrace;
}
```

Its port holds what it restores in a guard, `let _restore = Restore {
.. }`, whose `Drop` restores it, as Rust writes a `finally`. ADR 0437 made
a run of statements to the guard's drop one `try { .. } finally { .. }`,
but only a flat one: each statement in it a `try` whose `catch` drops the
guard, or one that can't throw. The port's scope has an `if`, loops in
it, and a `return frame;` from inside them, which MIR writes `drop(guard);
return frame;`. So each statement that may throw kept its own `try { .. }
catch (error) { ..the drop..; throw error; }`, about fifteen copies of the
four restores.

## Decision

**A run a drop ends may hold `if`s, loops and labeled blocks, and leave
from inside them**, where each way out drops the value first:

- **In it**: a `try` whose `catch` drops the value last; a statement that
  can't throw; an `if`, a `while` or a labeled block whose test can't
  throw, of such statements, nested as deep as they go; and a `break` or a
  `continue` of a loop or label in the run.
- **A way out of it** is the drop, then a `return` of nothing, of a
  constant, or of a variable the run declares and no closure in it sets,
  which the drop can't change; or a `break` or a `continue` of what's
  outside it. In the `try`, the drop goes: its `finally` runs it as JS
  leaves, after the `return`'s value is made, as Rust drops it after.
  `const v = e; drop; return v;` is `return e;`.
- **Not in it**, as before: another value's drop last, whose life crosses
  this one's; a statement that may throw outside a `try`; a drop not
  followed by a way out.

```js
reentry = true;
const previous = work(0);
const _restore = { previous };
try {
  const a = work(n);
  if (a > 3) {
    while (i < a) {
      ..
      if (tmp) {
        return result;
      }
    }
  }
} finally {
  reentry = false;
  work(_restore.previous);
}
```

## Why

- **It's React's `finally`**: one place says what the scope restores,
  however it's left, a throw, a `return` from a loop, its end.
- **It's shorter**: 34 corpus programs and two apps' snapshots lost about
  2,900 lines of `catch`es that each dropped and threw again; each shape
  `docs/readability.txt` counts is as many as before, a larger share of
  fewer lines.
- **The same drops**: every corpus case still matches native Rust, and
  `drop_guard_scope_returns` drops a guard on a `return` from a loop
  inside an `if`, at its scope's end and on a panic inside the loop.

## Alternatives

- **A flag the `finally` tests, `if (live) drop`**: also covers a drop not
  on every way out, at the cost of a flag React doesn't have.
- **Lowering the guard as a `finally` where MIR's drop is**: MIR gives each
  call its own cleanup; the run is found once the body is JS, as ADR 0437
  finds it.

## Consequences

- A guard struct's fields stay its object's, `const _restore = {
  previous }`, read in the `finally` as `_restore.previous`, where React's
  `finally` reads the variables themselves. A struct only its fields are
  read of, in place of its variables, is a rewrite of its own, not done
  here.
- A temporary a `try` held, `const tmp = f(i) === 7; if (tmp)`, stays
  apart where it's read: the `try`s go after tidy's reads in place.

## What holds it

- `test/lowering-unwinds.test.ts`, "a guard's scope is one finally, its
  returns in it", run in Node: a throw in the loop, a `return` from it,
  the scope's end, each restoring once.
- Corpus case `drop_guard_scope_returns` against native Rust.
- Mutations `scoped-drops-if-uncovered`, `scoped-drops-exit-kept`
  (`mir/tidy/unwinds.rs`).

## Amended (2026-10-25): around a `catch`, one `try`, `catch` and `finally`

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same statement.

React's `startTransition` is `try { .. } catch (error) {
reportGlobalError(error); } finally { .. }`. Its port's guard is dropped
around a `catch_unwind`, which was `try { try { .. } catch (error) { ..
} } finally { .. }`: the scope's `try` held the `catch_unwind`'s `try`
and `catch` alone.

**A `finally` whose `try` holds a `try` and `catch` alone is printed as
one `try`, `catch` and `finally`** (`to_oxc`'s spelling of
`StmtKind::Try`): JS runs the `finally` after the `catch` either way, and
after what either throws or returns. The tree keeps its two statements,
each of its own meaning to the passes before.

Held by `test/lowering.test.ts`, "a guard dropped around catch_unwind is
its try's catch and finally", run; the mutation
`try-catch-finally-nested` (`to_oxc/statements.ts`).

## Amended (2026-10-25): a function or a constant returned last is after the `finally`

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same value
returned and the same drop, run in the same order on every path; only
where the `return` is written changes.

React's `startGestureTransition` is one `try`, `catch` and `finally`,
then `return noop;`. Its port's scope ends with `&noop`, which Rust makes
before the guard drops, so the scope's run, `result = noop;` last, was
`try { try { .. } catch (error) { .. } return noop; } finally { .. }`
(`returned_through_finally`): the `try` held more than the `catch`, and
wasn't printed as one with it.

**What a scope returns last, the same whenever it's made, is returned
after its `finally`**: a constant, or one of the program's functions,
its own or a library's (`Expr::is_fixed`), `try { .. } catch (error) {
.. } finally { .. } return noop;`. Made before the drop or after, it's
the same value: the drop can't change a constant, nor what a function's
name holds, which nothing assigns. Anything else, a variable or a call,
is returned in the `try` as before, made before the `finally` runs.

What a function's reference is, lowering knows where it writes one
(`items.rs::fn_ref`), and says so on the expression, `Known::Item`; a
later pass reads that, not the name, which a module's `let` of a static
may share the form of, and a drop may set. One of the program's functions
is never `undefined` either (`Expr::may_be_undefined`): a nullable place
stores it as it is, `h.done = noop`.

Held by the corpus cases `catch_unwind_guard_returned` (React's
`startGestureTransition`: a guard, a `catch_unwind` returning what its
scope's `Option::map` gave, and a function after; each way out restoring
once) and `catch_unwind_nested_ok` (`return 7;`), against native Rust;
`test/crates.test.ts`, "a library's function returned past a guard's
drop is returned after its finally"; `test/lowering-nullable.test.ts`, "a
function a nullable place is given is stored as it is"; and the mutations
`fixed-return-kept-in-finally` (`lower/mir/tidy/unwinds.ts`),
`own-item-reference-unknown`, `library-item-reference-unknown`
(`lower/items.ts`), `item-not-fixed` and `item-may-be-undefined`
(`js/make.ts`).

## Amended (2026-10-26): the function's return place is given from inside the scope

Case ([0262](0262-when-rust-and-js-disagree.md)): A, the same value
returned and the same drop, run once, after the value's made, on every
path; only how the JS reads changes.

React's `runWithFiberInDEV` (ReactCurrentFiber.js) returns from inside
its `try`:

```js
try {
  if (arg0 !== undefined && ..) {
    return callback(arg0, ..);
  }
  return callback(arg0, ..);
} finally {
  setCurrentFiber(previousFiber);
}
```

Its port holds the previous fiber in a guard, `let _restore =
Restore(previous_fiber);`, and returns early from its scope, `return
callback(..);`, which MIR writes `result = callback(..); drop(guard);
return result;`. A way out of the run was a `return` only of a variable
the run declares, and `result`, the function's return place, is declared
at the function's top: the early return ended the run, so each way out
was a scope of its own, `let result; if (wrap) { try { result =
callback(); } finally { .. } return result; } try { .. } finally { .. }
return result;`, the drop written twice.

**A `return` of the function's return place, after the drop, is a way
out of the run wherever the place is declared** (`Run::leaves_after_drop`,
given `Hints::result`): only its own assignments set it, as Rust's
closures never capture the return place and no drop names it, so the drop
can't change it, and the `return` in the `try` gives what Rust returns.
The run then reads as React's, `const v = e; drop; return v;` being
`return e;` as before, and `returned_through_finally` the last:

```js
const previousFiber = current;
setCurrentFiber(fiber);
try {
  if (wrap) {
    return callback();
  }
  return callback();
} finally {
  setCurrentFiber(previousFiber);
}
```

Held by `test/lowering-drop-guards.test.ts`, "an early return in a
guard's scope returns in its try", run in JS: each way out, a throw too,
restores the fiber once; the corpus case `drop_guard_early_return`
(React's `runWithFiberInDEV`, nested and panicking) against native Rust;
and the mutation `return-place-out-of-scope` (`mir/tidy/unwinds.rs`).
