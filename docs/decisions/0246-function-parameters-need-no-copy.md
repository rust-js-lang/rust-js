# 0246. A type parameter of a function needs no copy of its own

Status: Accepted. Amends [0020](0020-structs-and-tuples.md).

## Context

react.dev's Challenge takes its props apart, a callback among them:

```js
export function Challenge({isRecipes, totalChallenges, currentChallenge, hasNextChallenge, handleClickNextChallenge}) {
```

Its Rust is generic in the callback, `F: Fn() + Copy`. A type parameter
may be a value changed in place, which a copy must not share, so rust-js
kept a struct holding one whole, `param.hasNextChallenge`, and read each
field where it is.

## Decision

**A type parameter bound by a function trait, `Fn`, `FnMut` or `FnOnce`,
holds nothing changed in place: a struct of one is taken apart, and a
copy of one is the function.**

```js
export function Next({ total, next }) {
```

- **It's what a closure's own type is already**: a JS function, which a
  copy of is the same function. So a `Copy` `FnMut` closure's copies
  share what it captured, where Rust's each have their own, as before.

## Why

- **It's the JS a person writes.**
- **It's tested**: a JSX test takes apart the props of a component
  generic in its callback, beside a type changed in place, which a type
  parameter could otherwise be, and calls it; the corpus runs as before. A
  mutation keeps them whole.
