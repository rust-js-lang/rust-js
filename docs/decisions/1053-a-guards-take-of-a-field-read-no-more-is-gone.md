# 1053. A guard's take of a field read no more is gone, and a comment runs nothing

Status: Accepted (rd429, P2, 2026-11-02). Amends
[0705](0705-a-guards-scope-reads-as-any-body.md) (a write of a guard's
field nothing reads after is gone) and
[0437](0437-a-scopes-drops-are-one-finally.md) (a scope's drops are one
`finally`).

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each rewrite runs the
same code in the same order. Nothing of Rust's differs.

## Context

React's Flight client's initializeModelChunk restores two module
variables however it's left:

```js
const prevHandler = initializingHandler;
const prevChunk = initializingChunk;
initializingHandler = null;
..
try {
  const value = parseModel(response, resolvedModel);
  ..
} finally {
  initializingHandler = prevHandler;
  if ((enableProfilerTimer && enableComponentPerformanceTrack) || __DEV__) {
    initializingChunk = prevChunk;
  }
}
```

In Rust this is a guard whose `Drop` takes each field back,
`INITIALIZING_HANDLER.set(self.prev_handler.take())`. rust-js printed the
guard as an object, `const _restore = {prevHandler: old, prevChunk: ..}`,
for two reasons.

First, a take writes the field, `_restore.prevHandler = undefined`. A
write that nothing reads after was already dropped (ADR 0705), but "read
after" meant any mention of the guard. The drop's next statement reads
`_restore.prevChunk`, a different field, so the write stayed, the field
became a `let`, and the guard stayed whole.

Second, the drop was printed twice. Once was in a `catch` around the
throwing call, which rethrew; the other was at the scope's end. The code
between them held an `if` whose branches are only comments, `if
(handler.errored) { // Ignore .. }`. A block of only comments ends in
`StmtKind::Comments`, which `only_makes_values` didn't count as making
values. So the scope's run wasn't covered, and no `finally` was made.

The Flight client printed 50 references to `_restore` this way.

## Decision

**A write of a guard's field is gone where nothing after reads that
field**, though another field is read (`drops::inlined`'s `GuardReads`:
the fields read after, by key, or the whole guard where it's named
otherwise). A guard read whole, given to a function, still keeps every
write.

**A block's end of only comments makes nothing and can't throw**, so a
scope's `finally` runs past it, as it runs past a `let` of nothing.

The Flight client's guard is then its fields, and the code after the call
is in the one `try`:

```js
const old = initializingHandler;
initializingHandler = undefined;
const prevChunk = initializingChunk;
..
try {
  value = parseModel(chunk);
  ..
} finally {
  initializingHandler = old;
  if (DEV) {
    initializingChunk = prevChunk;
  }
}
```

The handler's field reads as `old`, the variable it was made of: Rust's
`INITIALIZING_HANDLER.take()` is that `const` and its write. The port
names it upstream's way by binding it first, `let prev_handler =
INITIALIZING_HANDLER.take();`. A nested `Option`, `prev_chunk:
Option<Option<Chunk>>`, which is `None` where the chunk isn't saved, is
boxed, `$some(..)`. The port writes upstream's form instead: the chunk is
always saved, and its restore tests the flag, as upstream's `finally`
does.

## Consequences

- Upstream's `finally` form for the Flight client's guards, once the port
  writes them as above.
- Tests: `test/corpus/drop_guard_restores_taken_fields.rs` (native
  comparison, a panic through the guard included). Mutations:
  `guard-field-read-by-another`, `comments-block-scope`.
