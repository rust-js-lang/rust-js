# 0322. Text's and slices' methods by their other names, and a join of lists

Status: Accepted. Extends [0063](0063-text.md) and
[0315](0315-vec-capacity-and-edits.md); counted by
[0314](0314-std-data-structures.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)), and C for a join of
lists.

## Context

ADR 0314's count, with patterns tried, had `String` at 17 of its 43
methods, `str` at 45 of 83 and slices at 35 of 133. Some missing ones are
methods rust-js knows by another name: the deprecated `trim_left` and
`lines_any`, a `String`'s `as_bytes`, a `Box<str>`'s `into_string`. And
`join` of a slice of lists, `[vec![1, 2], vec![3]].join(&0)`, was JS's
string `join`: `"1,203"`, not `[1, 2, 0, 3]`.

## Decision

**A method that's another one by a different name is that one, and a
slice of lists joins their items.**

| Rust | JS |
|---|---|
| `trim_left`, `trim_right`, `trim_left_matches`, `trim_right_matches`, `lines_any` | `trim_start`'s and the rest's |
| a `String`'s `reserve`, `reserve_exact`, `shrink_to`, `shrink_to_fit` | nothing, as a `Vec`'s (ADR 0315) |
| a `String`'s `as_bytes`, `into_bytes`, a `Box<str>`'s `into_boxed_bytes` | `Array.from(new TextEncoder().encode(s))` |
| `into_boxed_str`, `into_string`, a `Box<[T]>`'s `into_vec` | the value |
| `String::from_utf8_unchecked(bytes)` | `$utf8Decode(bytes)`, as `str`'s (ADR 0172) |
| `items.join(sep)`, `connect`, of text | `items.join(sep)` |
| `lists.join(&x)`, `lists.join(&[x, y][..])` | `$joinWith(lists, x, false)`, `$joinWith(lists, [x, y], true)` |

- **A join of lists clones each item**, as `Join`'s impl does, by its
  clone function where a clone is more than the item.
- `connect`, deprecated, is `join`.

## Why

- **It's the same program**: each is the method it's named after, and a
  join of lists is `Join`'s.
- **It's tested**: a corpus case runs each against native Rust, a join of
  lists by an item, by a slice, and of `String`s changed after; mutations
  take lists for text and a slice separator for one item.
- `docs/std-coverage.txt`: `String` 25 of 43, `str` 52 of 83, slices 37
  of 133.
