# 0294. `unwrap_unchecked()` is the value, as TypeScript's `x!` is

Status: Accepted.

Case: A ([0262](0262-when-rust-and-js-disagree.md)): a program can't
observe the difference, since a `None` is undefined behavior.

## Context

TypeScript's `x!` says a value is there and checks nothing: react.dev's
Preview hands `iframeRef.current!` to Sandpack. Rust's `unwrap()` checks
and panics, so a port with it read `$unwrap(iframeRef.current)`: a helper
and a check the original doesn't have. rust-js refused `unwrap_unchecked()`.

## Decision

**`unsafe { o.unwrap_unchecked() }` is the value itself: `o`, or of an
`Option` of an `Option`, what its box holds (ADR 0051).** It's what `x!`
means, and a port writes it where the original asserts.

```rust
let iframeElement = unsafe { iframeRef.current().unwrap_unchecked() };
// const iframeElement = iframeRef.current;
```

## Why

- **It's the JavaScript a person writes**: no check where the original has
  none.
- **It's the same program**: Rust makes a `None` here undefined behavior,
  so whatever JS does with `undefined` is within what the program means.
  A port that wants the check keeps `unwrap()`, and `$unwrap`.
