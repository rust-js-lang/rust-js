# 0279. A callback that passes on what it's given is the function

Status: Accepted. Extends [0217](0217-enumerate-index.md). Amended by
[0605](0605-an-arrow-that-passes-on-has-one-owner.md): an event's handler
asks `passed_on` too; and by its amendment: an array method is
`passed_on.rs`'s `Taker::ArrayMethod`, and a function of rest arguments
keeps its arrow (below).

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

## Amendment: the owner's taker, and a function of rest arguments

An array method's callback is `passed_on.rs`'s `Taker::ArrayMethod` (ADR
0605, amended), whose list of methods is there, and reads MIR's fact of
what the closure calls with every other taker. A function of the crate's
that takes rest arguments, `#[rust_js::rest]`, isn't one that "takes no
more than it's given": `xs.map(|x| many(&[*x]))` was `xs.map(many)`,
which JS gives the index and the array too, so `many` counted three, where
Rust's counts one, a wrong answer. Its arrow stays, `xs.map((x) =>
many(x))`; the corpus's `once_callback_passed_on.rs` holds it against
native Rust.
