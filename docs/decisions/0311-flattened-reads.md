# 0311. `.flatten()` of a read of items that may be `None` is the read

Status: Accepted. Extends [0051](0051-generic-options.md): an `Option` of
what could look like `None`.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's Console reads a JS array of any values, `null` among them:
`args.shift()`, `consoleData.data[0]`. In Rust those are
`args.pop_front().flatten()` and `data.first().copied().flatten()`, an
`Option` of an `Option` whose inner one looks like `None` in JS, which
rust-js boxes (ADR 0051), and refused for `pop_front`.

## Decision

**`.flatten()` of a read of items that may be `None`, `pop_front`,
`pop_back`, `pop`, `first`, `get(i)` or `last`, through a `copied()` or a
`cloned()` of a value that's itself, is the JS read: `args.shift()`,
`args.pop()`, `items[0]`, `items[i]`, `items.at(-1)`.**

## Why

- **It's the same program**: JS's `undefined` is both no item and a `None`
  one, and `flatten` makes Rust's two `None`s one. Nothing is boxed to be
  unboxed.
- **It's the JavaScript a person writes**: react.dev's `args.shift()`.
- **It's tested**: a lowering test runs each read of items with `None`
  among them, of a deque, a slice and a `Vec`, empty too; a mutation
  refuses it again.
