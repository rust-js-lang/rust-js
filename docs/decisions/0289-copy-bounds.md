# 0289. A `Copy` bound takes a copy only where a copy isn't the value

Status: Accepted. Amends [0052](0052-std-trait-impls.md): a `T: Copy`
copies with its dictionary only where it may be given a value whose copy
is another object.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A `T: Copy` took a copy function, `TCopy`, and copied with it wherever a
`T` was used again (ADR 0052). Most callers give one whose copy is the
value itself: a number, text, a shared reference, an `Option` of one. For
them every call carried `{ copy: (value) => value }` and every use a
`TCopy.copy(x)`. react.dev's Preview debounces an `Option<&SandpackError>`
through `useDebounced<T: Copy>`, so `useDebounced(rawError)` became
`useDebounced(rawError, { copy: (value) => value })`, which no person
writes.

A copy matters only for an object changed in place: a `Copy` struct the
crate mutates (ADR 0052's changed types) is a new object each copy.

## Decision

**A type parameter's `Copy` bound takes a copy function only where it may
be given a value whose copy isn't the value: by a caller in the crate, by
a type parameter of the caller's own that is, or by a caller rust-js can't
see.**

```rust
fn twice<T: Copy>(value: T) -> (T, T) { (value, value) }
fn kept<T: Copy>(value: T) -> T { let copy = value; copy }

twice(name)                // twice(name), function twice(value)
kept(point)                // kept({ ...point }, { copy: (value) => ({ ...value }) })
```

- Its own copy: a shared reference, a number, `bool`, `char`, text, a
  function, a closure that changes nothing it captured, `()`, and an
  `Option` of one. Anything else, a type parameter's aside, gives one.
- A type parameter passed on, `relay<T: Copy>` calling `inner::<T>`, gives
  the callee one if the caller is given one.
- Every one is given a copy where callers aren't all seen: a trait's
  method and an impl's, called through dictionaries; a library's function
  its consumers can reach; a function of another crate.
- An impl's `T: Copy` counts as its method's.
- A trait impl's own `T: Copy` takes one too: its dictionary is given to
  callers rust-js can't see, as its methods are (found by rustc's
  `traits/conditional-dispatch.rs`).

## Why

- **It's the JavaScript a person writes**: `useDebounced(rawError)`.
- **It's the same program**: where no value changes in place, a copy and
  the value can't be told apart; where one may, its copy is still made.

## Consequences

- A function is given a copy function by one caller that needs it, and
  every caller then passes one, as with drops (ADR 0098).
