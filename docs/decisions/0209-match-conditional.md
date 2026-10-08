# 0209. A two-arm `match` that's a value is a conditional

Status: Accepted.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A `match` was always statements: one that's a value, `let kind = match
kind { Primary => "a", Secondary => "b" };`, was a `let`, its subject in
a `const match`, and an `if` that assigns it, where a person writes a
conditional, and where an `if` that's a value already is one. And a
`bool` matched against `true` was `match === true`, where it's the bool.

## Decision

**A `match` that's a value, of two arms that bind nothing, the first
guarded or not by a plain guard, is a conditional, its subject written
where the test reads it once, else in a `const`; and a `bool` tested
against `true` is the bool, against `false` its negation, in any
`match`.**

```js
const kind = (type ?? "primary") === "primary" ? "bg-link" : "text-primary";
const label = n === 0 && flag ? "flagged zero" : "other";
const look = css.length === 0 ? FRAME_STYLE : `<style>\n${css.join("")}</style>`;
```

- **A first arm that takes everything is the value**, its guard, if it
  has one, the test, the second arm never reached.
- **Arms may bind what a subject in place holds**, read only and owning
  nothing: each name is its place, as an `if let` or `matches!` binds it.
  `match item { Some(Item { path: Some(path), .. }) => path.as_str(), _ =>
  "none" }` is `item != null && item.path != null ? item.path : "none"`,
  and `Err(n) => n + 1` reads `r._0`. A binding that owns what it binds,
  a `mut` one, or one of a subject that isn't a place is statements, as
  before.
- **What its subject holds is as before**: a `&mut` number's value,
  `n.value === 0`, and a destructor's subject dropped in a `finally`.

## Why

- **It's what a person writes**, and what an `if` that's a value already
  is: one statement, `const`, no assignment in two places.
- **It's exact**: the test is the pattern's and the guard's, made in
  Rust's order, the subject made once, a body only if its arm is taken; a
  `bool` is a JS boolean, so `b === true` is `b`.
- **It's tested**: a compiler test runs each shape, a subject in place
  and in a `const`, a guard, a `&mut` number, a subject with a
  destructor, a first arm that takes everything, `true` and `false`
  arms; the corpus checks every program against native Rust.
