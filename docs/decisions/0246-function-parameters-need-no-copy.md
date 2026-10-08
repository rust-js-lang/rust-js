# 0246. A type parameter of a function needs no copy of its own

Status: Accepted. Amends [0020](0020-structs-and-tuples.md).

Case: A, D ([0262](0262-when-rust-and-js-disagree.md)).

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
  copy of is the same function, and no program can tell, as a copy of one
  that changes nothing it captured, `Fn`, is the same closure in Rust too.
- **A copy of a `Copy` closure that changes what it captured is an error**,
  `rust-js does not support copying a closure that changes what it
  captured yet`. Rust gives each copy its own captures, and a JS
  function's copy shares them, so one that could tell isn't made, as a
  clone of one isn't: a read of its variable but the variable's only use,
  as a move is, or a call, which lends it; and a `Copy` bound's
  dictionary for it, which a generic function copies by, as each call of
  a `Copy` `FnOnce` does.

  ```rust
  let mut f = move || { n += 1; n };
  f();
  let mut g = f;            // Rust: g has its own n, so f() and g() are 2 and 2
  ```

## Why

- **It's the JS a person writes**, where no program can tell.
- **A copy that could tell is refused, not shared** (ADR 0262): sharing
  printed `2 3` where Rust prints `2 2`. Keeping Rust's copy would make
  each such closure keep its captures apart, in an object of its own, for
  copies few programs make.
- **It's tested**: a JSX test takes apart the props of a component
  generic in its callback, beside a type changed in place, which a type
  parameter could otherwise be, and calls it; the corpus runs as before. A
  mutation keeps them whole. The corpus rejects a copy and a `Copy`
  bound's, and runs a move and a copy of an `Fn` closure; mutations
  allow each, or reject the move, the call and the `Fn` copy.
