# 0479. `reduce` is std's

Status: Accepted. Extends [0055](0055-iterator.md) and
[0051](0051-generic-options.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): the same value, and
`None` of none.

## Context

`Iterator::reduce` wasn't lowered: a call of it was refused. JS's own
`reduce` with no start throws of an empty iterator, where Rust's gives
`None`.

## Decision

**`it.reduce(f)` is `$reduce(it, f)`**: its first item, folded with each
after it by `f`, or `None` of none; boxed where its item may look like
`None`, `$reduce(it, f, true)` (ADR 0051). It's `fold`'s lowering from its
first item, `folded` of no start, and leaves no item with a destructor
undropped, each moved into `f`.

```js
const s = $reduce($iter([1, 2, 3]), (a, b) => (a + b) >>> 0);
```

## Why

- **It's std's**: `iterator_reduce` reduces numbers, text, `Option`s
  through generic code, and values with a destructor, and of none, with
  native Rust.

## Consequences

- **What compiles grows**: `reduce`.
