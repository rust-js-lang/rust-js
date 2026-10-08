# 0247. JS's string methods are the builtins crate's `string`

Status: Accepted. Extends [0102](0102-js-and-webapi.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

react.dev's code reads its strings by JS's indexes, UTF-16 code units:

```js
const label = isLead ? g.slice(0, -1).trim() : g;
args.push(query.slice(query.indexOf(']=') + 2));
```

Rust's `str` counts bytes, so its `find` and slicing are lowered to
helpers that convert, `$strSlice(query, ..)`, and its `trim` isn't JS's
quite. Each ported component declared JS's methods it used itself.

## Decision

**The builtins crate's `string` module has JS's string methods, named as
ReScript's standard library names them: one function for each form,
`slice` and `slice_to_end`, `substring` and `substring_to_end`,
`index_of` and `index_of_from`, `last_index_of`, `char_at`, `replace`, `trim`, `trim_start`,
`trim_end`, and `length`.**

```rust
let label = if is_lead { string::trim(&string::slice(g, 0, -1)) } else { g.clone() };
```

```js
const label = isLead ? g.slice(0, -1).trim() : g;
```

- **Their indexes are JS's**, signed where JS counts from the end.
- **A split or a replace by a pattern stays the program's**, as the
  `reg_exp` module has it: what JS gives, a group that didn't match too,
  is the pattern's.

## Why

- **It's the JS a person writes**, and one declaration, not one each.
- **It's tested**: a compiler test calls each on a string with an emoji
  and spaces around it, beside JS's own, and checks each is the method.
