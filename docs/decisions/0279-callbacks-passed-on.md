# 0279. A callback that passes on what it's given is the function

Status: Accepted. Extends [0217](0217-enumerate-index.md).

Case: B ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A Rust closure that calls a function with its arguments is an arrow of
them, where JS passes the function:

```rust
array.iter().enumerate().map(|(index, item)| renderCallback(item, index)).collect()
```

```js
array.map((item, index) => renderCallback(item, index));  // the original: array.map(renderCallback)
```

## Decision

**A closure given to an array method, `map`, `forEach`, `filter` and the
rest, that only calls a function with its own arguments, in order, is the
function**, where the function is Rust's: one of the crate, not a binding,
or a closure, a generic `impl Fn`'s too. A binding's JS function,
`parseFloat`, keeps its arrow.

## Why

- **It's the JS a person writes.**
- **Rust's take no more than they're given**: the method gives the index
  and the array too, which a function of the crate, or a closure, has no
  parameter for. A JS function might read them, `parseInt`'s radix, so a
  binding isn't passed.
- **Where it's observable, rarely**: a generic `impl Fn` a JS caller gives
  may be a JS function of more parameters than its type says, which would
  read the index and the array.
- **It's tested**: a test passes a function of the crate and an `impl Fn`,
  and keeps a binding's arrow; mutations keep each arrow, and pass the
  binding.
