# 0588. A binding may throw by the result it's declared with

Status: Accepted. Amends [0035](0035-results-and-throwing-js.md)'s fallible
bindings.

Case ([0262](0262-when-rust-and-js-disagree.md)): A fix: results changed,
so Rust's is kept.

## Context

A binding says it may throw by its result: one returning a `Result` runs
in a `try`, `$try(() => f(x))`, and one returning `Promise<Result<..>>`
settles, `$settle(p)` (ADRs 0035, 0442). Which it was came from the
signature with the call's type arguments in it, so a generic binding given
a `Result` was taken to say so:

```rust
let (a, b) = js::promise::all2((js::promise(parse(a)), js::promise(parse(b)))).await;
```

`js::promise<T>(future) -> Promise<T>` of an `async fn parse() ->
Result<u32, String>` was `$settle(parse(a))`: its fulfilled `Result`
wrapped in another `Ok`, so `a?` gave the inner `Result` as a number, and
the sum was `0` where Rust's is `3`. Writing the guide found it.

## Decision

**A binding declared to give a type parameter, `T` or `Promise<T>`,
never says it may throw, whatever `T` is given** (`Recognition::catching`):
`promise<T>(..) -> Promise<T>` is its promise. One declared with a
`Result`, `settle<T>(..) -> Promise<Result<T, &JsError>>`, says so, as one
whose trait's associated type is one does, `stringify_with`'s
`T::Stringified`, where the impl the call picks says which values may
throw.

## Why

- **It's the binding that says it**: a `Result` the binding's author
  wrote, or the trait's impl gives, is a throw caught; a `T` a caller
  fills with a `Result` is that caller's value, which JS gives as it is.

## What holds it

- `test/lowering.test.ts`, "a generic binding given a Result is not one
  that may throw": `all2` of two `js::promise`s of `async fn`s that give
  `Result`s, run, and no `$settle`.
- `test/bindings.test.ts`, "react.dev's Console": `stringify_with` of a
  value that may throw, still a `try`.
- Mutations: a type parameter's `Result` taken to throw; the associated
  type read unfilled.
