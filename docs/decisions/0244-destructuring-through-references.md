# 0244. A value taken apart through a shared reference is destructured

Status: Accepted. Extends [0020](0020-structs-and-tuples.md) and
[0023](0023-strings-references-shared-state.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's ErrorDecoder takes its params apart as it gets them:

```js
const {errorMessage, errorCode} = useErrorDecoderParams();
```

Its Rust gets a `&'static ErrorDecoderParams`, and a pattern of a
reference binds each part by reference. rust-js kept the value whole and
read each part where it is, `tmp.errorMessage`, as it does a place that
might change. So did each closure that takes a pair apart by reference,
`|&(path, _)|`, `(param) => param[0] === target`.

## Decision

**A tuple or struct pattern of a shared reference is JS's destructuring:
each part bound by reference is read once.**

```rust
let Params { message, code } = params();
files.iter().find(|(path, _)| path == shown)
```

```js
const { message, code } = params();
files.find(([path]) => path === shown);
```

- **What's borrowed can't change while it's borrowed**, so a part read
  once is the part read where it is, for as long as it's bound.
- **A `Cell`, which can change, is one JS object** (ADR 0099), the same
  one either way.
- **A `ref mut` part, and a place taken apart in place, are as before.**
- **A closure inlined where it's called reads its parts**: `|&(_, age)|
  age` of `item` is `item[1]`, not the arrow called.

## Why

- **It's the JS a person writes.**
- **It's tested**: a compiler test takes a struct apart through a call's
  reference, one holding a `Cell`, which it sets, and a key closure
  inlined; the corpus and the playground's snapshots run and show each
  closure's pair destructured.
  Mutations keep the reference whole, bind each part where it is, and
  call the inlined closure.
