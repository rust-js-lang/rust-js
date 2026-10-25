# 0603. A comment before a `let` with no JS is the next statement's

Status: Accepted. Extends [0444](0444-a-comment-is-where-its-code-is.md)'s comments.
Extended by 0444's amendment of 2026-10-25: a `let` of a block, `unsafe {
.. }`, too, and the start after them may be part of the next `let`'s code.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: a comment changes
nothing a program does.

## Context

A comment is printed before the statement it's before, where only that
statement's start is between them (ADR 0444): one before a struct, which
has no JS, is nobody's. React's scheduler's Rust is

```rust
// Timer fired. Transfer to the task queue.
let timer = Rc::clone(timer);
pop(timer_queue);
```

whose `let` has no JS, `timer` read in place, so the comment was lost
where upstream has it before `pop(timerQueue);`.

## Decision

**A comment before `let`s that have no JS of their own is the next
statement's**: what's between is only `let ..;`, with no block. A `let`
read in place gives its comment to the code that reads it next. What's
before an item with no JS, a struct, stays nobody's.

## What holds it

- `test/lowering.test.ts`, "a comment before a let that has no JS is the
  next statement's"; three corpus cases' comments are now printed.
- Mutation: the comment dropped.
