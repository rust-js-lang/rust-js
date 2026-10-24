# 0744. A value or what it falls back to is `||`

Status: Accepted. Extends [0364](0364-mir-lowering.md)'s branches of two
values.

Case ([0262](0262-when-rust-and-js-disagree.md)): it can't be observed:
`!x ? y : x` and `x || y` give the same value of every `x`, and an `x`
that does nothing is the same read once as twice.

## Context

React's `addValueToProperties` (shared/ReactPerformanceTrackProperties.js)
wraps a value in its promise's description:

```rust
inserted_entry.1 = "Promise<".to_string()
    + if inserted_entry.1.is_empty() { "Object" } else { &inserted_entry.1 }
    + ">";
```

A string's emptiness is its falsiness in JS, so the `if` of two values was
`!insertedEntry[1] ? "Object" : insertedEntry[1]`, where upstream writes
`insertedEntry[1] || 'Object'`.

## Decision

**`!x ? y : x`, and `x ? x : y`, of an `x` that does nothing, a variable or
a path of fields and indexes, is `x || y`** (`conditional`, which makes each
branch of two values tidy writes as one).

## Consequences

- The port's `'Promise<' + (insertedEntry[1] || 'Object') + '>'` is
  upstream's.
