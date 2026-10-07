# 0233. A `match` giving a table's field named as each variant reads the table by it

Status: Accepted. Builds on [0013](0013-fieldless-enums.md).

## Context

react.dev's ExpandableCallout keeps each callout's look in one object,
`variantMap`, and reads it by the callout's type, `variantMap[type]`.
Rust reads a struct's field by name only, so a port matches the type:

```rust
match kind {
    Kind::Note => &VARIANTS.note,
    Kind::Pitfall => &VARIANTS.pitfall,
    Kind::Rsc => &VARIANTS.rsc,
}
```

which was a conditional of every variant:

```js
let tmp;
if (kind === "note") {
  tmp = VARIANTS.note;
} else if (kind === "pitfall") {
```

## Decision

**A `match` whose every arm is one variant, whose value is its name, and
gives the field of that name of one table, `&MAP.note` or `MAP.note`, is
the table read by what's matched: `MAP[kind]`.**

```js
return VARIANTS[kind];
```

- **Rust says each variant has its arm**: a `_` arm, an or-pattern, a
  guard or a binding is a `match` as before.
- **Each arm is a place that stays put, read alone**: an arm of another
  field, `Kind::Note => &MAP.pitfall`, or of another table, is a
  conditional as before.
- **What isn't a place, `match pick(i)`, is read once, as the key**:
  `VARIANTS[pick(i)]`. The table is a place, so reading it first changes
  nothing.

## Why

- **It's the JS a person writes**: one read of the table, as react.dev's
  `variantMap[type]` is.
- **It's exact**: a fieldless enum's value is its variant's name (ADR
  0013), the same string the field is named, so `MAP[kind]` is the arm's
  field for every value.
- **It's tested**: the corpus's `match_index` runs a table read by a
  variable and by a call beside native Rust, and arms of another field
  and of another table; a compiler test checks which is `MAP[kind]`.
