# 0292. An index a condition shows in bounds is read as it is

Status: Accepted. Narrows where `$index` checks an index.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`v[i]` panics past the end, where JS reads `undefined`, so it's
`$index(v, i)`, which throws as Rust panics. Only a constant index of an
array whose length its type says was read as it is. react.dev's Preview
reads the first lint error where it knows there is one:

```ts
if (lintErrors.length === 0) {
  return null;
} else {
  const {line, column, message} = lintErrors[0];
```

Its Rust, `if lint_errors.is_empty() { None } else { .. &lint_errors[0] ..
}`, gave `$index(lintErrors, 0)`: a helper where the original reads the
item.

## Decision

**Of a `Vec` or a slice and an index the function never changes, an index
a condition shows in bounds is read as JS reads it: `xs[0]`.**

What shows it, and where:

- In an `if`'s branch, what its condition shows held, or didn't, there:
  `!xs.is_empty()`, `xs.len() > k`, `xs.len() >= k`, `xs.len() == k`,
  `xs.len() != 0` show the indexes below `k`; `i < xs.len()` shows `i`.
  `!`, `&&` that holds and `||` that doesn't, combine them.
- After `if c { return ..; }`, what `c` not holding shows, to the end of
  its block.
- In `for i in 0..xs.len()`, `i`.

"Never changes": nothing in the function sets it, lends it as `&mut`, or
binds it `ref mut`, in whole or in part. A closure's body is its own: it
knows only what its own conditions show.

## Why

- **It's the JavaScript a person writes**: `lintErrors[0]` after the test.
- **It's the same program**: where the condition showed the index in
  bounds, `$index` could never throw, and nothing changed since.

## Consequences

- Most indexes stay checked: an index a condition doesn't show, as
  `while i < xs.len()` with `i += 1`, is `$index`.
