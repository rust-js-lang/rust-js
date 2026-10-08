# 0278. A variable nothing sets again is a `const`

Status: Accepted.

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

A `let mut` is a JS `let`, though often nothing sets it again: it's only
changed in place, as react.dev's toCommaSeparatedList's list is.

```rust
let mut list: Vec<Box<dyn ReactNode>> = Vec::new();
list.push(..);
```

```js
let list = [];      // the original: const list = [];
list.push(..);
```

## Decision

**A `let` with a value that nothing in its function sets again, nor a
`&mut`'s handle, is a `const`.** One set again, or declared and set later,
`let n;`, stays a `let`. Setting a field, `list.length = 0`, isn't setting
the variable.

## Why

- **It's the JS a person writes**: `const` says the name holds one value,
  which a reader then needn't look for.
- **Nothing changes**: a `const` and a `let` set once are the same JS.
- **It's tested**: a test keeps a pushed `Vec` a `const`, and a counted
  number, a variable set in both branches, and one a `&mut` is taken of,
  `let`s; mutations make all `let`s, and a set one a `const`, which JS
  refuses.
