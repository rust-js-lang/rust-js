# `xor` drops what it gives up

Status: Draft. Would extend [0496](../0496-and-drops-what-it-gives-up.md),
whose `and` drops its argument by the one table of what std leaves
unused, and [0490](../0490-a-value-std-leaves-unused-is-dropped-there.md).

Case: C ([0262](../0262-when-rust-and-js-disagree.md)): what's dropped,
and when, as std's.

## Context

`Option::xor` of values with a destructor is refused: `Some(a).xor(Some(b))`
is `None`, and std drops both `a` and `b` as it returns; `Some(a).xor(None)`
is `Some(a)` and drops nothing. ADR 0496 gave `and` its drops from one
table, `std_leaves_unused`, of what each std combinator drops of its
subject and its argument, by which is `Some`.

## Decision

**`xor` is a row of ADR 0496's table: both `Some`, its subject's payload
and its argument's are dropped, the subject's first, as `Option::xor`'s
match drops them; one `Some`, nothing; neither, nothing.**

```js
const r = $xor(a, b, loudDrop_drop);
```

- **No new form**: `kept_or` has the shape, a helper that takes the drop
  and gives the kept value; `xor` is one more.
- **Of a type parameter's values**, the drop given to the function is the
  one passed, as `and`'s is.

## Why

- **It's std's**: a corpus case `xor`s each of the four pairs, of values
  with a destructor and of a type parameter's, with native Rust.
- **It's the table's**: one place says what std leaves unused, and `xor`
  joins it.

## Consequences

- itertools' and the memory's last `Option` combinator refusal goes.
