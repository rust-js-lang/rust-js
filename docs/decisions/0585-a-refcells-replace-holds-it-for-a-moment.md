# 0585. A `RefCell`'s `replace` and `take` hold it for a moment

Status: Accepted. Extends [0362](0362-refcell-fields-never-counted.md),
whose field's `RefCell` is its value where each borrow of it is held for a
moment, and [0580](0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A borrow that can't
fail isn't counted; one that can still is, and still throws where Rust
panics.

## Context

React's scheduler takes a task's callback before it runs it, and puts back
what's left of it, a field upstream sets:

```rust
let callback = current_task.callback.take();
..
current_task.callback.replace(Some(continuation_callback));
```

A field's `RefCell` was its value only where each borrow of it was a
guard held for a moment (ADR 0362); one lent to `replace` or `take` was a
cell, so every task's callback was `{ value }`, each borrow counted:

```js
const callback = $cellReplace($borrowMut(currentTask.callback));
```

Upstream's is `const callback = currentTask.callback; currentTask.callback = null;`.

## Decision

**A `RefCell`'s `replace(v)` holds it only while it swaps the value, and
so is a borrow held for a moment**: `v` is made before, and the old value
drops after, the borrow ended. `take()` too, where `T::default()`, which
runs while it's held, is std's. A field's `RefCell` that's only borrowed
so is its value, and each is a read and a write:

```js
const callback = currentTask.callback;
currentTask.callback = undefined;
..
currentTask.callback = continuationCallback;
```

So, by the one place each answers it:

- **`CellBorrows`** takes a `RefCell`'s `replace`, `take`, `swap` and
  `replace_with` as borrowing it mutably, where they borrowed nothing (a
  `swap`'s other cell is lent, so counted already): a
  guard held across a call that replaces its cell is counted, and the
  `replace` throws as Rust's panics.
- **A `Cell`'s and a plain `RefCell`'s `replace` and `take`** are the old
  value read and the new one written, where `$cellReplace` did both; a
  counted `RefCell`'s are `$cellReplace($borrowMut(..))`, as before.
- **An old value nothing reads is no statement**: tidy drops `const old =
  task.callback;` that nothing reads where the next statement writes that
  place, `task.callback = next;`, as `mem::replace`'s was too. Another
  unread read stays, a binding's getter's, `el.offsetWidth`, which runs.
- **Whether `T::default()` is std's is recognition's** (`std_default`).
- **Whether a `RefCell` is plain is asked of its type, its regions
  erased** (`is_plain_ref_cell`), by lowering and declarations alike: a
  field's type says `dyn FnMut() + 'static`, a body's `'{erased}`, and
  only one of them was found.

`replace_with` stays a cell's, its `f` running while it's held; `swap`
lends a second cell, which is a cell then.

## Why

- **It's what std does**: `replace` takes its borrow, swaps, and returns;
  nothing of the program runs inside it.
- **It reads as written**: a task's callback is the function, set and
  read as upstream's.

## Alternatives

- **Write `*task.callback.borrow_mut() = v`** in the port: Rust bent for
  rust-js, and its old value drops inside the borrow, which may ask.
  Rejected.

## Consequences

- Five corpus cases' cells are their values, `let count = 5`; renames grow
  from 838.3 to 839.4 in 10,000 lines, each an old value nothing binds,
  `old$1`, where `$cellReplace(..)` was read in place.
- `let x = cell.take()?` still names the value `old`, as `Option::take`
  does; a binding that takes it in an `if let` names it.

## What holds it

- `test/lowering.test.ts`, "a field's RefCell written by replace and take
  is its value": take, run, replace, `is_none`, cancel, as React's
  scheduler does; one replaced across a borrow, which throws; one taken of
  the crate's `Default`, a cell.
- Mutations: `replace` taken to borrow nothing, a field lent to it a cell,
  a `take` of the crate's `Default` trusted, a plain one borrowed, the
  unread read kept, regions kept in the lookup.
