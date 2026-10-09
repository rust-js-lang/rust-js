# 0301. A read that can't throw comes before a move

Status: Accepted. Amends [0197](0197-moved-before-leaving.md): what may
come before the move.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A value moved before anything in its scope can leave has no flag and no
`try` (ADR 0197), and, since ADR 0300, a function that drops nothing else
of its `T` takes no drop for it. react.dev's `useEvent` reads its ref, then
calls with its arguments:

```ts
return useCallback((...args) => {
  const f = ref.current!;
  return f(...args);
}, []);
```

Its port, `let f = unsafe { r#ref.current().unwrap_unchecked() }; f(args)`,
kept `args` owned while it read the ref: a binding's call may throw, which
unwinds as a panic, so `args` had a flag, a `try`, and `useEvent` a `dropA`.
But `ref.current` is a data property, as @types/react declares
`RefObject<T> { current: T }`, and a read of one can't throw; nor can
`unwrap_unchecked`, which never checks.

## Decision

**A binding marked `#[rust_js::cannot_throw]` can't leave, nor can
`unwrap_unchecked()`; and a `let` whose value can't leave may come before
the move that ADR 0197 looks for.**

```rust
let f = unsafe { r#ref.current().unwrap_unchecked() };
f(args)
// const f = ref.current; return f(args);
```

- The react crate marks `RefObject::current`. A getter may throw, as
  `localStorage`'s does, so a binding isn't one unless marked.
- A `let` whose value is what ADR 0098's analysis says can't leave, a
  literal, a variable, a field, and the calls above, declares what may
  come first.
- A pointer coercion can't leave either: the `&dyn Fn` a ref holds is
  coerced to the closure's narrower lifetime as it's read, and a box to a
  `Box<dyn ..>` as it's made. (Amended as it was done: react.dev's
  `useEvent` kept its `try` until it was.)

## Why

- **It's the JavaScript a person writes**: `useEvent(fn)`, no `try`.
- **It's the same program**: nothing before the move can leave, so the
  drop its flag guarded can't run.
