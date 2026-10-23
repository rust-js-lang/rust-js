# 0703. A closure made before a call a guard unwinds is made where it's read

Status: Accepted. Extends [0270](0270-module-variables.md), whose
`.with(|c| *c)` is the thread-local's variable, and
[0437](0437-a-scopes-drops-are-one-finally.md)'s `try` of a call that
may panic as something's there to drop.

Case ([0262](0262-when-rust-and-js-disagree.md)): 1, the program can't
observe it. Making a closure does nothing and can't throw, so where it's
made, outside the call's `try` or in it, changes nothing that runs.

## Context

React's `describeNativeComponentFrame`, ported, reads its cache twice:

```rust
if __DEV__ && let Some(cache) = COMPONENT_FRAME_CACHE.with(|cache| *cache) {
    cache.set(r#fn, frame.clone());
}
```

Past its guard's scope it was `if (__DEV__ && componentFrameCache)`. In
it, where a panic drops the guard, it was

```js
const value = cache => cache;
let weakMap;
try {
  weakMap = value(componentFrameCache);
} catch (error) { /* the guard's drop */ throw error; }
```

A call that unwinds elsewhere than what's made before it first makes
those (`structure.rs`), so they run outside its `try`: of the closure
too, which `with`'s call then called by name, not applied in place.

## Decision

**What's made before such a call runs first only where it may throw**
(`may_throw_first`): a closure made, `cache => cache`, is made where it's
read, so `with` applies it in place, `componentFrameCache`, and nothing's
left to try. **What a `try` would hold that can't throw, first, is before
it** (`unwinding`): a closure two calls read, `const key = (n) => n[0];`,
then the `try` of the call it's given to.

## Why

- **One form of `.with(|c| *c)`**: its variable, under a guard or not.
- **The program can't observe it**: neither making a closure nor reading
  a variable throws, so no `catch` can see it.

## Alternatives

- **Apply a closure by name, `value(cache)`, in tidy**: it's in a `try`
  by then, and moving a call out of one is a pass of its own. Rejected
  for not making the call at all.

## Consequences

- Corpus snapshots of a closure given to a call under a guard, a
  `sort_by_key`'s, a `retain`'s, read it where it's given, or made before
  the `try` as a `const`.

## What holds it

- `test/lowering-cells.test.ts`, "a thread-local's with is its variable
  under a guard", run in JS, its guard dropped on a panic.
- `test/corpus/vec_destructors.rs`.
- Mutations `closure-made-throws-first`, `throwing-made-not-first`,
  `unwinding-made-first-tried`.
