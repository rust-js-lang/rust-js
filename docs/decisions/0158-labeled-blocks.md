# 0158. A labeled block is JS's labeled block

Status: Accepted. Extends [0015](0015-loops.md).

## Context

A labeled block leaves early with a value, as a function returns early,
without being a function:

```rust
let found = 'search: {
    for row in rows {
        if let Some(x) = row.find(..) {
            break 'search Some(x);
        }
    }
    None
};
```

It was an error.

## Decision

**`'name: { .. }` is JS's `name: { .. }`, and `break 'name value` gives
`value` to where the block's value goes, then `break name`,** as a loop's
`break value` does:

```js
let found;
search: {
  for (const row of rows) {
    ..
    found = x;
    break search;
  }
  found = undefined;
}
```

- **A block whose value a function returns** returns it: each `break
  'name v` is `return v`, and no label is written.
- **JS needs the label even from just inside it:** a bare `break` leaves
  only a loop.
- **A loop inside one keeps its own `break` and `continue`,** as Rust's
  does: an unlabeled one is the loop's.
- **A `break` names the block itself,** its `hir::Block`, which THIR's
  `region_scope` for the block is, not the expression around it.

## Why

- **It's exact:** the `labeled_blocks` corpus case compares values,
  nesting, a block left from a loop inside it, and a loop inside one with
  native Rust.
- **It's the JS a person writes:** JS has the same construct.
