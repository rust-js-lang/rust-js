# 0300. A function takes the drops its body uses

Status: Accepted. Amends [0098](0098-destructors.md) and
[0100](0100-separate-crates.md): which generic functions take a drop.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A generic function took a drop for its `T`, `dropT`, where a caller in the
crate gave it a value with a destructor (ADR 0098), and a library's, for
each `T` that isn't `Copy`, as its consumers may (ADR 0100). Cargo builds
every crate as a library, the app too, so react.dev's `useEvent` took two:

```js
export function useEvent(fn, dropA, dropR) { .. }
```

`R` is only returned: nothing in its body drops one. Nor does `keep(value)`,
which moves its value into what it returns, or `relay(value)`, which passes
it on to `keep`. Each took a drop, and each caller passed one.

## Decision

**A function given drops by name, not through a dictionary, keeps only
those its body uses: to drop a value, or passed on to a function that
keeps it.** Whether it drops one is the lowering's own answer, where its
drops are written: the drop of what it owns as it ends, normally or by a
panic, of an old value assigned over, of what a closure in it holds.

```rust
pub fn keep<T>(value: T) -> Kept<T> { Kept(value) }
pub fn pick<A, B>(a: A, _b: B) -> A { a }
pub fn discard<T>(value: T) { pick((), value); }
```

```js
export function keep(value) { return [value]; }
export function pick(a, _b, dropB) { try { return a; } finally { dropB?.(_b); } }
export function discard(value, dropT) { pick(undefined, value, dropT); }
```

- Each call marks the drops it gives, which the pipeline keeps or leaves
  out once every function is lowered: a function's drop passed on as
  another's is kept where that one keeps it.
- A library's manifest says which it keeps, so a consumer gives those.
- A trait's method, and an impl's, take what they took: a dictionary's
  callers can't see which of its impls they call.

## Why

- **It's the JavaScript a person writes**: a drop where something drops.
- **It's the same program**: a drop no code reads, given or not, runs
  nothing.

## Consequences

- A function that may panic before it moves its value still drops it, and
  takes a drop for it: `useEvent` reads its ref before it calls.
