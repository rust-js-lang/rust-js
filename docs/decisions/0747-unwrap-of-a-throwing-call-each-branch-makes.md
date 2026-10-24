# 0747. `unwrap()` of a throwing call each branch makes is each call

Status: Accepted. Extends [0643](0643-unwrap-of-a-throwing-call-is-the-call.md),
of one call, to a `Result` each branch of an `if` or a `match` makes.

Case ([0262](0262-when-rust-and-js-disagree.md)): B, as 0643's: what's
observable is JS's own error where Rust's panic wraps it, rare and
harmless, its Consequences say where.

## Context

React's `addValueToProperties` (shared/ReactPerformanceTrackProperties.js),
ported, unwraps the JSON of one of two arrays:

```rust
break 'desc if did_truncate {
    js::json::stringify_with(js::unknown(&[&array[..OBJECT_WIDTH_LIMIT], &[Some(js::unknown("…"))]].concat()), None, 0)
} else {
    js::json::stringify_with(value, None, 0)
}
.unwrap()
.unwrap();
```

rust-js wrote `$unwrap($unwrapOk(didTruncate ? $try(() =>
JSON.stringify(..)) : $try(() => JSON.stringify(value, null, 0)),
undefined, String))`, where upstream writes `JSON.stringify(..)` and lets
JS's throw go. ADR 0643's `unwrap()` is the bare call where the `Result`
is a `$try(..)` made in place; here MIR makes it in each branch, into a
local the `unwrap()` reads, so its `unwrap()` saw a variable.

## Decision

**A throwing call's `Result` unwrapped where it's made, of each branch
that gives it, is each call itself**, and the `unwrap()` its value:
`if c { f(a) } else { f(b) }.unwrap()` is `c ? f(a) : f(b)`.
`unwrapped_where_made` answers it of a local, and both sides ask it: the
call, which is then the call, uncaught, and the `unwrap()`, which is then
the value. It holds where:

- **the local's one read is an `unwrap()`** of it, not an `expect(msg)`,
  whose message ADR 0643 keeps;
- **each write of it is a call that throws**, as `call_throws` says: a
  branch that makes its `Result`, `Ok(None)`, or calls a Rust function,
  keeps `$try`'s, its `unwrap()` `$unwrapOk`'s;
- **nothing runs between each call and the `unwrap()`**: each block on
  the way only goes on, or drops what drops nothing. A `Result` held
  while something else runs, a `println!`, keeps `$try`'s, as 0643 has it,
  so the throw is caught where it's made, before what follows.

So React's reads `desc = $unwrap(didTruncate ? JSON.stringify(..) :
JSON.stringify(value, null, 0))`.

**The outer `$unwrap` stays.** `stringify_with` of an `Unknown` gives
`Option<String>`, as `JSON.stringify` of a function or `undefined` is
`undefined` (ADR 0310), and the program's second `unwrap()` is Rust's
check that it isn't: a `String` that is `undefined` would change what
follows. Upstream's array is never one, but `Unknown`'s type can't say
so; a `Stringify` of an array of any values, whose JSON is always text
unless it throws, would, and is left for its own decision.

## Why

- **It's upstream's JS**, each `JSON.stringify(..)` bare, no helper.
- **One rule**: what 0643 says of one call, of the call each branch
  makes, by one owner both sides ask.

## Alternatives

- **Read the conditional tidy makes, `c ? $try(a) : $try(b)`, and take
  its `$try`s off**: a later pass reconstructing what lowering decided,
  from its JS, and telling `unwrap()` from `expect(msg)` by its
  arguments.
- **Write the port's `unwrap()` in each branch**: Rust bent for the
  compiler.

## Consequences

- As 0643's: only JS that catches the throw, of a `pub` function a JS
  caller calls, sees JS's own error where Rust's panic is an `Error`
  that says it.

## What holds it

- `test/js-builtins.test.ts`, "unwrap of a throwing call each branch
  makes is each call", run in JS: two `JSON.parse`s and two
  `JSON.stringify`s each bare, and a branch of `Ok(None)`, of a Rust
  function, of a `println!` past the call, and of a temporary whose
  `Drop` prints past it, each `$try`'s.
- Mutations `unwrapped-where-made-never`, `unwrapped-past-what-runs`,
  `unwrapped-of-any-call`, `unwrapped-of-an-assigned-result` and
  `unwrapped-where-made-call-caught`.
