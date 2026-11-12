# 0179. `filter` and `map_or` drop what they don't keep

Status: Accepted. Extends [0098](0098-destructors.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`filter` of an `Option` keeps its value only if a test says so, and
`map_or` uses its fallback only if there's no value:

```rust
d.and_time(time).map_or(MappedLocalTime::None, MappedLocalTime::Single)
```

What either doesn't keep, Rust drops: `filter`'s value once the test says
no, `map_or`'s fallback once the function has run. Both were an error of
a value with a destructor, which stopped chrono, at 8 places.

## Decision

**What `filter` doesn't keep, and `map_or`'s unused fallback, are dropped
where Rust drops them, and if the test or function panics:**

```js
let kept;
try {
  if (option != null && even(option)) {
    kept = option;
  }
} finally {
  if (option != null && kept === undefined) {
    dDrop_drop(option);
  }
}
```

```js
let mapped;
if (option != null) {
  try {
    mapped = f(option);
  } finally {
    dDrop_drop(fallback);
  }
} else {
  mapped = fallback;
}
```

- **Only of a value with a destructor:** one with none is the expression
  it was, `option != null && even(option) ? option : undefined`.
- **The value moves into `map_or`'s function,** which drops it, as
  `map`'s does.
- (Amended: the other combinators that give their value to a function
  do too, `is_some_and`, `is_none_or`, `and_then`, `map_or_else`,
  `unwrap_or_else`, and a `Result`'s `map_err`, `and_then`, `map_or_else`
  and `unwrap_or_else`; and what a `Result`'s combinator gives up is
  dropped: `is_ok_and`'s `Err`, `is_err_and`'s `Ok`, `ok()`'s `Err`,
  `err()`'s `Ok`, and `map_or`'s `Err`, or its fallback once `f` has run.
  The `combinators_owned_drops` corpus case runs each against native
  Rust.)

## Why

- **It's exact:** the `option_combinator_drops` corpus case compares
  `filter` kept, not kept and of `None`, `map_or` of a `Some` and a `None`,
  each in generic code too, with native Rust.
- **It's what Rust's own `filter` and `map_or` do:** the value is theirs,
  dropped as they return.

## Amendment (2026-11-12, #1505): what `unwrap` panics over is dropped

A `Result`'s `unwrap`, `expect`, `unwrap_err` or `expect_err` that panics
dropped nothing of the value it panicked over. Std's is `Err(e) =>
unwrap_failed("..", &e)`: `e` is a local of `unwrap`'s frame, which the
panic unwinds out of, so it's dropped there, after its message is made,
caught or not. Now, where that value has a destructor, `$unwrapOk` and
`$unwrapErr` are given its drop after their `debug`, `$unwrapOk(r,
undefined, loudDebug_fmt, loudDrop_drop)`. They make the message, then
run the drop under `$dropsUnwinding`, where a second panic aborts as
Rust's does, then throw. A generic function's is the drop its callers
give, `dropE`. Results change, so Rust's behaviour is kept (ADR 0262's
third row). The coordinator approved the fourth argument.

Proof: the corpus case `unwrap_panic_drops_what_it_panics_over`, against
native Rust in both builds. It covers each of the four caught, in
generic code, a value whose drop glue drops two, a box, and one uncaught
as `main` unwinds. The metamorphic net's `loud` rewrite of
`unwrap_typed_debug` found it. Snapshots that gain the drop:
`channel_of_droppers`, `drop_parts_past_a_panic`, `try_lock` (a
`TryLockError::Poisoned`'s guard, released).
