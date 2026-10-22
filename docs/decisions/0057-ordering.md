# 0057. `PartialOrd` and `Ord`: an `Ordering`, and the parts in turn

Status: Accepted. `Reverse` compares the other way round, and another crate's struct is an error rather than field by field ([0068](0068-queues.md)). Extends [0036](0036-iterators-and-sorting.md) and [0053](0053-partial-eq.md).

Case: C, D ([0262](0262-when-rust-and-js-disagree.md)).

## Context

An `Ordering` is -1, 0 or 1 (ADR 0036), and JS's `<` compares numbers the
way Rust does. Nothing else could be compared:

- `<` on strings, `bool`s or structs;
- `cmp` of a struct;
- sorting a `Vec` of them;
- a hand-written `impl Ord`, or a generic `T: Ord`.

`sort_by_key` compared keys with `$cmp`, which is JS's `<`, so for a
tuple key it silently didn't sort.

## Decision

**`a.cmp(&b)` is an `Ordering`, and `a.partial_cmp(&b)` is one too, or
`undefined` for `None`** (`Some(x)` is `x`, ADR 0030).

| The type | `cmp` |
|---|---|
| numbers, strings, `char`s, `bool`s, `Ordering` | `$cmp(a, b)` |
| `f64`, `partial_cmp` | `$partialCmp(a, b)`: `undefined` if either is `NaN` |
| a hand-written impl | `wordOrd_cmp(a, b)` |
| a generic `T` | `TOrd.cmp(a, b)`, or `TPartialOrd.partial_cmp(a, b)` |
| derived, a struct or tuple | `$cmp(a.major, b.major) \|\| $cmp(a.minor, b.minor)` |
| derived, a fieldless enum | `$cmpIn(["Low", "Mid", "High"], a, b)`, by discriminant, as the derive compares: `High = 2, Low = 1` is `["Low", "High"]` (ADR 0089 found the declaration order used before) |
| `Option` | `None` first, then the values |
| `Vec`, slices, arrays | `$cmpItems(a, b, cmp)`: item by item, then shorter first |

- **Derived: the parts in turn, joined with `||`.** `Equal` is 0, the one
  falsy `Ordering`, so `||` gives the first part that isn't equal.
- **Where a part can be unordered** (an `f64`, or anything that isn't
  `Ord`), `undefined` is falsy too, so the parts go through `$thenCmp`
  instead, which stops at it.
- **`a < b` is `cmp < 0`,** and the same for `<=`, `>` and `>=`. Strings
  and `bool`s use JS's own `<`. For an unordered pair, `undefined < 0` is
  false, as every comparison is in Rust.
- **`a.max(b)` is `cmp > 0 ? a : b`, and `a.min(b)` is `cmp > 0 ? b : a`.**
  On a tie, Rust's `max` returns `b` and its `min` returns `a`.
- **Sorting:**
  - `v.sort()` is `v.sort(cmp)`, which is stable, as Rust's is;
  - `sort_by_key` compares the keys with their `cmp`;
  - an iterator's `max` and `min` are `$maxBy(items, cmp)` and
    `$minBy(items, cmp)`: the last of the greatest and the first of the
    least, as in Rust.
- **Dictionaries, as ADR 0049 has them:** `T: Ord` gets `{ cmp }` and
  `T: PartialOrd` gets `{ partial_cmp }`. A derived impl's dictionary is
  its comparison, or `$cmp` itself.
  - **Only an `Ord` or `PartialOrd` bound:** `a == b` is `cmp === 0`
    rather than a third dictionary.
  - **A generic `max()` boxes its `Some`** if it would look like `None`
    (ADR 0051).
- **Numbers stay as they were:** JS's `<`, `Math.max`, `$cmp`.
- **Still errors:** comparing enums with fields, and `clamp`.

## Why

- **The plain case stays short.** A derived `Ord` is the comparison a
  person would write, `||` included, with no helper call.
- **`NaN` behaves.** An `f64` field makes the struct's `<` false both ways,
  as Rust's does, instead of quietly treating `NaN` as equal.
- **One function, `cmp_value`, serves operators, `max`, sorting and
  dictionaries alike,** so they can't disagree.

## Alternatives

- **A named function per derived impl** (`semverOrd_cmp`). Each comparison
  would be a call rather than an expression. It would read better where a
  struct with many fields is compared often, and it could come later.
- **Always `$thenCmp`.** It's correct everywhere, but it evaluates every
  part when `||` stops at the first difference, and it reads worse.

## Amendment: a struct of no fields

`struct Zst;` has one value, `Equal` to itself, as its derive has it, and
so has `()`, a heap of which is compared so (ADR 0068): its
comparison is `0`, which, as a part of a struct ordered field by field,
drops out (ADR 0426). It was an error, as a struct whose shape had no
parts, and stopped rustc's `zero-sized-btreemap-insert`, a `BTreeMap` keyed
by one, and `obligation-error-propagation`. The `unit_struct_order` corpus
case orders one as a map's and a set's key, compared, and as a field, with
native Rust.

## Amendment: a sort of what can be `None`

JS's `sort` moves each `undefined` to the end without asking the
comparator, so a sort of `Option`s, whose `None` is `undefined`, put
`None` last, where `Option`'s `Ord` puts it first (#101): `sort`,
`sort_by`, `sort_by_key`, and a `BTreeSet<Option<_>>` listed. Items that
can be `undefined`, those `can_be_nullish` names, are sorted by `$sort`,
which sorts their indexes, never `undefined`, by the same comparator and
puts the items in that order: `$sort(scores, (a, b) => …)`. Every other
sort is JS's own `items.sort(…)`, as before; `sort_by_compare` is the one
place that picks. ADR 0262's case: results would change, so Rust's. Held
by `test/corpus/option_sort.rs` and the mutations `nullable-sort-by-js`
and `sorted-keys-by-js`.
