# 0591. `is_some_and` of an equality with its `Some` is the equality

Status: Accepted. Extends [0030](0030-option.md)'s `Option` and
[0051](0051-generic-options.md)'s unboxed `Some`.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. The two test the
same: a `None` is `undefined`, which no value of an unboxed `Some`'s type
is, so it's never `===` one.

## Context

React's scheduler asks whether the task it ran is still first:

```rust
if peek(task_queue).is_some_and(|first| Rc::ptr_eq(first, &current_task)) {
```

`is_some_and(p)` was `option != null && p(option)`, its subject held to be
read twice, and in an expression a closure called in place:

```js
const option = peek(taskQueue);
if (option != null && option === currentTask) {
```

Upstream's is `if (peek(taskQueue) === currentTask)`.

## Decision

**`is_some_and(p)` of an `Option` whose `Some` isn't boxed, where `p` is
an `===` of its argument with what doesn't read it, is that `===` of the
subject**: `peek(taskQueue) === currentTask`. The `Some`'s type can't be
`undefined` nor `null` (its `Some` would be boxed, ADR 0051), and what it's
compared with is of that type, so a `None` fails it as the `!= null`
did. The subject is made first, then what it's compared with, as Rust
does.

## Why

- **It's the test the program asks**: "is the first one this task".

## What holds it

- `test/lowering.test.ts`, "is_some_and of an equality with a defined
  value is the equality": `Rc::ptr_eq`, a number's `==`, a `>` kept, and
  a predicate that doesn't read its argument kept.
- Mutations: the `!= null` kept, a predicate that doesn't read it taken
  for one that does.
- No mutation of `!boxed`: no `===` of a boxed `Some` is lowered, a
  generic `T`'s `==` being its dictionary's; it's kept so that one never is.
