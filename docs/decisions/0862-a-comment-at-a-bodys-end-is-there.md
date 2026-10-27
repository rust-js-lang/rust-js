# 0862. A comment at a body's end is there, before its `None` or after its last code

Status: Accepted. Extends [0753](0753-a-branch-of-only-comments-keeps-them.md),
whose block of only comments kept them where it ends, to a block whose
comments come after its last code, and to a function's `None` given at its
end, which is no `return;`. Amended below: a `try`'s, a `catch`'s and a
`finally`'s block too.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): comments, which
nothing runs.

## Context

React's `countChildren` says why its callback gives nothing:

```js
mapChildren(children, () => {
  n++;
  // Don't return anything
});
```

Its port gives the `None` the callback's type asks for, with the comment
before it:

```rust
&|_, _| {
    n.set(n.get() + 1);
    // Don't return anything
    None
}
```

A comment is its next statement's (ADR 0444), and the `None` a function
gives at its end is a `return;` that isn't written: it went, and its
comment with it. So did a comment after a body's last code, `out.push(0);
// Nothing to do.`, which ADR 0753 kept only of a block of nothing else.

## Decision

- **A function's `None` given at its end, a comment before it, is
  `StmtKind::Comments` where the `None` is**, which lowering makes as it
  makes MIR's `_0 = None` (`sources::commented`); the `return;` goes, and
  the comment is printed where it was, before the body's `}`.
- **A block whose comments come after its last code ends with them**, as a
  block of only comments does (`sources::comments_end`): `out.push(0); //
  Nothing to do.` at the function's or the branch's end.
- **A function's body reaches its comments**, as a block's does: its span,
  to them, so oxc prints them inside its `}` (`commented_end`, one answer
  for both).

```js
export function found(n) {
  if (n > 3) {
    return n;
  }
  // Nothing found.
}
```

## Why

- **It's React's JS**, its comment where it wrote it.
- **A comment is where its code is (ADR 0444)**: here, the end of a body.

## Alternatives

- **`return; // Don't return anything`**: a `return` the program didn't
  write, which rust-js leaves out (ADR 0299).

## What holds it

- `test/lowering-comments.test.ts`, "a comment before a None given at a
  function's end, or after its last code, is at its end": a closure's, a
  function's after an early `return`, and a branch's and a function's
  after their last code, each run.
- Mutations `tail-none-comments-dropped`
  (`scripts/mutations/lower/mir/assign.ts`), `comments-end-only-comments`,
  `commented-never` (`lower/sources.ts`) and
  `function-body-comments-outside` (`to_oxc/statements.ts`).

## Amendment: a `try`'s and a `catch`'s block too

React's BadMapPolyfill ends the `try` it writes with `/* eslint-enable
no-new */`, which its port writes as the last line of the closure it
gives `catch_unwind`. The closure's body is the `try`'s (ADR 0660), its
comments at its end with it, but the printer gave a `try`'s, a `catch`'s
and a `finally`'s block no span, as it gave other blocks theirs
(`commented_end`), so the comment went to the next statement there was:

```js
  new Set([frozenObject]);
} catch (_e) {
  /* eslint-enable no-new */
  // TODO: Consider warning about bad polyfills
  hasBadMapPolyfill.value = true;
}
```

and a comment after a `catch`'s last code went after the whole `try`.
**A `try`'s, a `catch`'s and a `finally`'s block reach the comments they
end with, as any block does** (`clause_block`, of `commented_end`'s
answer), and starts there too, as oxc prints what's at a `catch`'s
block's start before its `{`:

```js
  new Set([frozenObject]);
  /* eslint-enable no-new */
} catch (_e) {
```

Test: `lowering-comments`'s "a comment at the end of catch_unwind's
closure or its Err's block is at its block's end", in what a module runs
when it's loaded and in a function, each run. Mutations
`clause-block-comments-outside` and `clause-block-starts-at-comments`
(`to_oxc/statements.ts`).

## Amendment: before a nullable function's `return null;` (2026-10-27)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, comments alone.

React's getActivityInstanceFromFiber ends `// TODO: Implement this on
ActivityComponent.` then `return null;` (c10 of the port's gaps). Its
port's tail `None`, of a `#[rust_js::nullable]` result, is no `return;`
that goes but `return null;`, where the `None` is. The marker of where its
comments end (`StmtKind::Comments`, above) stayed before it, and a block's
comments of only comments are printed after its code (ADR 0753):
`return null; // TODO ..`.

**Where a statement is printed at the marker, the comments are that
statement's, before it**: tidy's `source spans` drops a marker followed by
a statement that starts where it is, but a `return` of nothing, which
goes. A tail `None` that prints nothing keeps its marker, as before.

Held by `test/lowering-comments-ends.test.ts`, "a comment before a tail
None is before its return", run; mutations
`comments-before-tail-return-kept` and, of a `return;` of nothing, which
isn't printed, `comments-before-unprinted-return-dropped`
(`lower/mir/tidy.ts`).
