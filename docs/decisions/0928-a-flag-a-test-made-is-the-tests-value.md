# 0928. A flag a test made is the test's value

Status: Accepted. Extends [0659](0659-tidy-is-a-list-of-stages-with-contracts.md)'s
`chains` stage.

Case: A ([0262](0262-when-rust-and-js-disagree.md)): no program can tell.

## Context

trekhleb/javascript-algorithms' Matrix, ported, tests a matrix's format
with a `matches!` of a guard:

```rust
if !matches!(m, Matrix::Rows(rows) if matches!(rows.first(), Some(Matrix::Rows(_)))) {
    panic!("Invalid matrix format");
}
```

MIR gives the `matches!` a flag, a block `bb: { if (t) { x = true; break
bb; } x = false; }`, whose test is one only once `chains/and tests` puts
its parts together. `branch values`, which makes a block's value of each
branch, has run by then, so `chains/default values` gave the flag its
`false` first:

```js
let tmp = false;
if (m.TAG === "Rows" && m._0[0] && m._0[0].TAG === "Rows") {
  tmp = true;
}
if (!tmp) {
```

## Decision

**A flag MIR's block gives, `true` where its test holds and `false`
where it doesn't, is the test's value** (`branches::flag_values`, the
`chains` stage's `flag values`, before `default values`):

```js
const value = m.TAG === "Rows" && m._0[0] && m._0[0].TAG === "Rows";
if (!value) {
```

`!!t` where `t` is no `bool`, `!t` of the values the other way, as
`branches::conditional` makes them.

## Why

- **No program writes a `bool`'s value with an `if`**: clippy's
  `needless_bool` says `if c { true } else { false }` is `c`. So only what
  MIR made is folded, and no `if` a program wrote.
- **Not `branch values` again**: run in `chains`, it folds every block's
  value, React's scheduler's `startTime`, which upstream writes with an
  `if`, among them.

## What holds it

`test/corpus/flag_of_a_guarded_match.rs` and the mutation
`flag-given-first`.
