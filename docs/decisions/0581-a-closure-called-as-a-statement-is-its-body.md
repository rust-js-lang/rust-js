# 0581. A closure called as a statement where it's made is its body

Status: Accepted. Extends [0270](0270-module-variables.md), whose plain
thread-local's `with_borrow` is its closure given the variable, and
[0444](0444-a-comment-is-where-its-code-is.md)'s comments.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. Each rewrite
changes nothing that runs.

## Context

A plain thread-local's `with_borrow_mut(|queue| ..)` is its closure
called with the variable (ADR 0270), as ADR 0580 now makes React's
scheduler's queues:

```js
function advanceTimers(currentTime) {
  ((timerQueue) => {
    while (true) {
      const timer = peek(timerQueue);
      ..
          ((taskQueue) => {
            push(taskQueue, timer);
          })(taskQueue);
```

`calls::apply` made a closure's value of a call where its body is one
`return`; one of statements stayed called where it was made. Upstream's
reads the queue it has: `push(taskQueue, timer)`.

## Decision

**A closure called as a statement where it's made, its value unread, of
a variable or a path that reads the same, is its body in its place**
(`js::applied_blocks`), its parameter read as what it's given. Not:

- where the body's own declarations, at its top, are another name of the
  function's, or the variable given;
- where its parameter is set, or a closure in its body could shadow it;
- where a `return` in it gives a value.

A `return;` in it, which leaves the closure, stays where the call ends the
function's body, and is the loop's `break;` where the body ends with a
loop it's in and in no other loop; else the closure stays.

**A comment before such a call is before its body**: what's between a
comment and the statement it's before may open a closure, `|queue| {`, as
well as start that statement (ADR 0444); and a blank line before a
statement is a line of nothing between, not a closure's opening line nor
an attribute's.

```js
function advanceTimers(currentTime) {
  // Check for tasks that are no longer delayed and add them to the queue.
  while (true) {
    const timer = peek(timerQueue);
```

## Why

- **It's what the program does**: the closure is `with_borrow_mut`'s way
  to lend the variable, and in JS the variable is there to read.
- **Of JS, not of a construct**: what's rewritten is a call of an arrow
  made in place, wherever plain thread-locals make it; it runs where they
  are made plain, after lowering.

## Alternatives

- **Inline it in a labeled block**, `block: { .. break block; }`, for any
  `return;`: JS no one writes for a loop's exit. Rejected for the loop's
  own `break`.
- **Leave a closure's body to tidy**: tidy runs before the thread-local is
  known plain. Rejected.

## Consequences

- Six corpus cases and two snapshots read their thread-local in place;
  the playground's loses a blank line a doc comment's attribute made.

## What holds it

- `test/lowering.test.ts`, "a plain thread-local's closure called as a
  statement is its body": a `push`, a loop's `return;` its `break;`, one
  that ends the function kept, comments before each, run.
- Mutations: the closure kept, its `return;` not broken, a tail one
  refused, its parameter unread, the comment lost, a closure's opening a
  blank line.
