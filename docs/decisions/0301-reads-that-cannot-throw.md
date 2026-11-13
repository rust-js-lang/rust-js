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

## Amendment (2026-11-13, #1666): whether std's call can unwind is read from its MIR

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so Rust's
behavior is kept. A std call named `len`, `is_empty`, `is_some`, `is_none`,
`is_ok` or `is_err` lost its unwind edge by its name (`pure_std`'s
`Question`). But `Range::is_empty` and `RangeInclusive::is_empty` run their
`Idx`'s `PartialOrd`, which is the program's and may panic. When one did, the
values live across the call weren't dropped: a guard's `drop` never ran.
**Now `analysis/unwinding.rs`'s `std_call_never_unwinds` reads it from the
call's MIR, as it resolves.** Every call in that MIR is followed to what it
resolves to, the `Idx`'s order included: an intrinsic never unwinds, a std
function with MIR is followed in turn, at most four deep. Anything else may
unwind: the program's code, a body without MIR, a drop, an assertion. Each
call is followed whatever unwinding it says, because std's MIR is wasm32's,
a target that aborts, where every call says it never unwinds. No snapshot
moves: `len()` and the like still never unwind. The name-based
`std_never_unwinds`, which asks what rust-js's own runtime does (a print
never fails), stays for its own question; folding both into std's MIR is
#1671's change 2. Corpus cases range_is_empty_of_a_panicking_order_unwinds
and range_inclusive_is_empty_of_a_panicking_order_unwinds against native
Rust; mutations `std-question-by-name` (`mir/prepared.ts`) and
`std-mir-calls-unfollowed` (`analysis/unwinding.ts`), caught.
