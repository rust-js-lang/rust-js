# 0704. A module variable an `if` makes is assigned only where it's something

Status: Accepted. Amends [0601](0601-a-module-variable-an-if-makes-is-a-let.md),
whose `let name;` each branch assigns.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. `let name;` is
`undefined` already, so assigning it `undefined` again changes nothing.

## Context

React's componentFrameCache is

```js
let componentFrameCache;
if (__DEV__) {
  componentFrameCache = new PossiblyWeakMap();
}
```

Its port's `thread_local!` is `if __DEV__ { Some(js::weak_map::new()) }
else { None }`, and rust-js wrote

```js
let componentFrameCache;
if (__DEV__) {
  componentFrameCache = new WeakMap();
} else {
  componentFrameCache = undefined;
}
```

ADR 0601's `assigned_in_place` made each `return v` of the initializer's
closure `name = v`, `None`'s `undefined` too.

## Decision

**A `return undefined` at an end is no assignment** (`prepare.rs`'s
`assigned`), and an `else` left with nothing is gone; a `then` left with
nothing is the `else`'s, under the test negated, `if (!(t)) { x = v; }`.

## Why

- **Upstream's**: `let x; if (c) { x = v; }`.
- **Nothing changes**: each path assigns its variable once, at its end,
  so one that assigns `undefined` leaves it as `let` made it.

## What holds it

- `test/lowering-cells.test.ts`, "a thread-local an if makes None of is
  assigned only where it's something", run in JS.
- `test/lowering-cells.test.ts`, "a thread-local of a build global's if
  is assigned under the if as written": the port's componentFrameCache,
  `if __DEV__ { Some(if js::is_defined(..) { .. } else { .. }) } else {
  None }` of an `extern` static, is `if (__DEV__) { componentFrameCache
  = typeof WeakMap !== "undefined" ? new WeakMap() : new Map(); }`.
- Mutations `assigned-undefined-written`, `assigned-empty-then-kept`.
