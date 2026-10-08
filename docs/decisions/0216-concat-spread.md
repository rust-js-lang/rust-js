# 0216. A slice's `concat` of parts written out is an array of them, spread

Status: Accepted. Extends [0062](0062-combinators.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`[path.as_slice(), &[last]].concat()` is how Rust writes "these, then one
more", where TypeScript writes `[...path, last]`. rust-js kept the parts
first, as it keeps any value a method reads more than once, and flattened
them: `const result = [path, [last]]; result.flat()`. react.dev's
`getBreadcrumbs` is one.

## Decision

**A slice's `concat` of parts written out is an array of them, spread, and
a part written out as an array its items in place:**

| Rust | JS |
|---|---|
| `[path.as_slice(), &[last]].concat()` | `[...path, last]` |
| `[v, w].concat().len()` | `[...v, ...w].length` |
| `vec![vec![1, 2], vec![3]].concat()` | `[1, 2, 3]` |

- **The same value:** each part of a slice's `concat` is an array, which
  `.flat()` spread too, in the same order.
- **Of strings, still `join("")`**, a string.
- **`...items` is an array's item only**, in the JS rust-js writes: an
  optimization that takes an array's items apart, `[x][0]` as `x`, or JSX's
  children item by item, leaves one with a spread as it is.

## Why

- **It's what a person writes**, and it keeps no variable the Rust has no
  name for.
