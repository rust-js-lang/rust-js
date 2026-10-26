# 0217. `enumerate()` then a callback's method is the method, given each index

Status: Accepted. Extends [0062](0062-combinators.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

## Context

`enumerate()` is a `map` of each item to its pair, `.map((x, i) => [i,
x])`, which the next method takes apart: `words.map((x, i) => [i,
x]).map(([i, w]) => ..)`. JS's own array and iterator methods give their
callback each item's index already: react.dev's `Breadcrumbs` writes
`breadcrumbs.map((crumb, i) => ..)`.

## Decision

**`enumerate()` then `map`, `for_each`, `any`, `all` or `flat_map`, whose
closure takes each pair apart, is that method of the items themselves,
its callback given the item, then the index:**

| Rust | JS |
|---|---|
| `words.iter().enumerate().map(\|(i, w)\| ..)` | `words.map((w, i) => ..)` |
| `words.iter().enumerate().any(\|(i, w)\| ..)` | `words.some((w, i) => ..)` |
| `v.iter().enumerate().map(\|(i, _)\| ..)` | `v.map((_, i) => ..)` |

- **The same values:** an array's methods, and an iterator's, give the
  callback each item and its index, from 0, as `enumerate` counts.
- **Not `filter` or `find`**, which give back what they're given, the
  pairs, and a closure that keeps a pair whole, `|pair|`.

## Why

- **It's what a person writes**, and makes no pair JS takes apart again.

## Amendment: `fold`, and an item left out (#333)

`enumerate().fold(start, |acc, (i, x)| ..)` was `.map((x, i) => [i,
x]).reduce((acc, [i, x]) => .., start)`: found porting
trekhleb/javascript-algorithms' `classicPolynome`, which writes
`coefficients.reduce((accumulator, currentCoefficient, index) => .., 0)`.
**`fold` is `reduce` of the items themselves, its callback given the
accumulator, the item, then the index**, from 0 as `enumerate` counts,
JS's `reduce` with a start giving them so (ADR 0262 case A). And a pair
whose item is left out, `|(i, _)|`, which reaches the callback as `[i]`,
was paired too, for `map` as this decision's table has it: it's `(_, i)`
again, or `(_, __, i)` of `reduce` where the accumulator is `_`, a JS
parameter list binding a name once. Held by the corpus case
`enumerate_fold` and the mutations `enumerate-fold-paired`,
`enumerate-item-left-out-paired` and
`enumerate-unread-item-named-as-accumulator`.
