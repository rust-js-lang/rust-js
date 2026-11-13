# 1139. A closure gives back what Rust's does: a spawning one `undefined`

Status: Accepted (2026-11-13, #1685). Supersedes
[0257](0257-spawning-closure-async-arrow.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's behavior is preserved.

## Context

ADR 0257 wrote a closure of `()` whose body only spawns an async block,
`|| js::spawn(Box::new(async { .. }))`, as that block's async arrow,
`async () => { .. }`, on the ground that nothing reads a `()`. JS does:
React reads what an effect gives back as its cleanup, and calls a promise
on unmount, which throws. A binding's or a framework's caller may read any
callback's result, and Rust's `()` says nothing of whether JS ignores it.

## Decision

A closure gives back what Rust's does. One of `()` that only spawns is a
function that starts the block and gives back `undefined`:

```js
() => {
  (async () => { await copy("#58C4DC"); })();
}
```

An `async` closure's body is still its async arrow, as Rust's gives a
future (ADR 0029).

## Consequences

- `test/lowering-functions.test.ts`'s "a closure that only spawns an
  async block gives back undefined" calls such a handler and finds
  `undefined` at once, and what it spawned done after. Mutation
  `mir-spawning-closure-an-async-arrow` (`lower/mir/closures.ts`) writes
  the async arrow again; `mir-spawning-closure-arrow` and
  `mir-spawning-closure-directive-lost`, of the rewrite, are gone.
- React's port changes where a handler or an effect only spawns: each is
  upstream's function around an async call, where upstream writes the
  async arrow (react-in-rust's regeneration on this change).
- The guide's `js::spawn` section says so.
