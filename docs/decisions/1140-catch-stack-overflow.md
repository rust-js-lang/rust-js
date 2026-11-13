# 1140. A JS stack overflow is caught where a program asks: `js::catch_stack_overflow`

Status: Accepted (2026-11-13, #1692, the user's decision). Amends
[0262](0262-when-rust-and-js-disagree.md)'s amendment of the stack's
depth and [0692](0692-a-thrown-error-is-a-panic-and-what-js-threw.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): Rust's behavior can't
be preserved, a decided exception at the JS boundary. A JS engine's stack
is some ten thousand calls where Rust's main thread has 8 MB, so a
recursion Rust finishes may overflow in JS; how deep a call goes is its
input's, so no compile error can say where.

## Context

Since #1193, a JS stack overflow is Rust's abort: exit 134, no drop run,
no `catch_unwind` takes it. React's Fizz renders a tree by recursion, and
recovers from one too deep for the stack by catching the engine's
`RangeError` and rendering the rest again from a task of its own (its
`renderNode`'s `x.message === 'Maximum call stack size exceeded'`), each
`catch` and `finally` on the way restoring what it set. The port says the
same with `catch_unwind`, restoring and `resume_unwind`ing as upstream's
`catch` does, which native Rust never reaches: React's port aborted, and
its ReactDOMFizzServer and ReactDOMFizzShellHydration suites, 196 tests,
ran none.

## Decision

Inside `js::catch_stack_overflow`, a stack overflow unwinds as a panic
whose payload is the engine's error; outside one, it's #1193's abort.

```rust
pub fn catch_stack_overflow<F: FnOnce() -> R + UnwindSafe, R>(f: F) -> Result<R, &'static JsError>
```

The js crate's, beside `js::spawn`: `Ok` of what `f()` gives, or `Err` of
the engine's overflow, the type the js crate gives what JS threw
(`eval`'s, `decode_uri`'s). Its bound is `catch_unwind`'s.

- **Inside one, an overflow is a panic:** each frame's drops run as it
  unwinds, a `RefCell`'s guard and an `Rc`'s count among them; a
  `catch_unwind` takes it, its payload the engine's `&JsError`, and
  `resume_unwind` gives it back; what no `catch_unwind` takes is the
  catcher's `Err`. So a program's catch-and-rethrow, upstream's `finally`
  written as `catch_unwind` and `resume_unwind`, restores what it set as
  upstream's does. The innermost catcher catches.
- **A drop that may panic and overflows as it unwinds aborts,** with the
  overflow's lines, as a panic's second does.
- **None open, it's Rust's abort,** as #1193 made it.
- **Natively it's `Ok(f())`:** an overflow aborts the process, so it never
  gives `Err`.
- **The scope is the call's,** lexical and synchronous: what the program
  does with its `Result` runs after it returns, outside it, and so does
  work it spawns; an overflow there aborts.

Two limits, which no JS can do better at, in the user's words:

- Where the overflow leaves a frame no room to run its drops, those drops
  may be skipped, or, for a drop that may panic, the program aborts.
- A frame whose calls rust-js knows can't panic skips its drops as an
  overflow passes: such a frame has no unwinding path, sound in Rust,
  where an overflow aborts.

The rule React passed its suites on before #1193, when an overflow was a
panic everywhere, is this one, within a catcher. A first draft let no
`catch_unwind` take the overflow: every `catch_unwind` the port writes
for upstream's `finally` (fifteen in ReactFizzServer.rs) then skipped its
restore, and needed a second, overflow-only copy. Rejected, and with it
a one-`try` form of the catcher around a `catch_unwind`, which under this
rule never catches.

### Its JS

`$catchStackOverflow(f)`, a `Result`, which counts the catchers open,
`$overflowCatchers`, as the runtime's unwinding reads; a `catch_unwind`'s
`$catchable` tells an overflow inside one first, with no call, where the
deepest frame has little room. A program that never calls it is as it
was: its JS byte for byte, and the runtime's unwinding the same where no
catcher is open.

## Consequences

- `test/stack-overflow.test.ts`: an overflow caught unwinds as a panic,
  its frames' drops run where they may panic, a finally's drop the
  program sees too, and the cell's guard released; a `catch_unwind`
  inside takes it and `resume_unwind` gives it back; the innermost
  catches; `Ok` where nothing overflows; an overflow in what's done with
  the `Result`, a drop that may panic and overflows as it unwinds, and a
  `catch_unwind` with no catcher open, each Rust's abort. Mutations leave
  a catcher open, let no `catch_unwind` take it, skip a finally's drops,
  and abort a drop's overflow as a panic's. Where nothing overflows it's
  `Ok(f())`, a panic in it a panic either `catch_unwind` takes, as
  natively; the corpus, which links no js crate natively, has no case.
- React's port opens one catcher, in performWork, around the work it
  does; its upstream-shaped code is otherwise unchanged. Fizz is 195/196:
  "can recover from very deep trees during resume" fails in the whole
  file and passes alone. It's room, not state, as it moves with Node's
  `--stack-size` (passing at 2000 KB): replayElement's `catch` consumes
  the slot, `replay_nodes.borrow_mut().remove(i)`, where the deepest
  frame has under one frame of room, and its JS,
  `$removeAt($borrowMut(replayNodes), i)`, makes two runtime calls where
  upstream's `replayNodes.splice(i, 1)` makes one. A borrow used as a
  temporary and `Vec::remove` written as upstream writes them is its own
  change, next.
- The guide's `interop/exceptions.md` has it, and `semantics.md` its
  limits.
