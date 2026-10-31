# 1040. A `position` only tested is JS's own search: `indexOf`, `findIndex`, `-1` its `None`

Status: Accepted (rd316, the user's decision). Decides what
[0966](0966-a-floats-order-is-its-comparison.md) left open, and extends
[0576](0576-what-a-bodys-integers-can-be.md) and
[0642](0642-a-place-found-in-a-string-is-js-units.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference: the `Option` is the function's alone, so nothing else reads
its `-1`, and JS's searches step the array as `position` steps it.

## Context

React's test renderer edits a parent's children by `indexOf` and
`splice`:

```js
const index = parentInstance.children.indexOf(child);
if (index !== -1) {
  parentInstance.children.splice(index, 1);
}
parentInstance.children.push(child);
```

The Rust an engineer writes is `children.iter().position(|c|
Rc::ptr_eq(c, &child))`, or by `js::unknown(c) == js::unknown(&child)`,
then `children.remove(index)` or `children.insert(index, x)`. rust-js
printed `$position(children, (c) => c === child)`, `index != null`, and
`$removeAt(children, index)`: helpers no person writes, and a different
error where `children` isn't an array. ReactTestRenderer-test's "should
warn if used to render a ReactDOM portal" renders a portal into the test
renderer, so `children` is an `HTMLCollection`, and asserts upstream's
`indexOf is not a function`; `$position` steps the collection instead and
fails later, at `push` (gap rd316, P1).

## Decision

**A slice's `position` whose `Option` is the function's alone, only
tested and its index read, is JS's own search of the array, and its
`None` is `-1`** (`mir/positions.rs`, a body fact `LocalFacts` keeps):

- **Which**: a `position` of a slice's `iter()` made for it, stepped by
  nothing else, whose `Option` is written there alone and read only by its
  discriminant and its `Some`'s index (`queries::tested_and_taken`, the
  one rule an integer's `try_from` asks too, #366). The array is then the
  `iter()`'s value as it is, no `$iter`.
- **Its form**: `children.indexOf(child)` where the test is `(c) => c ===
  x` of an `x` that reads the same each time, `Rc::ptr_eq`'s and an
  identity `==`'s; else `children.findIndex(f)`. `findIndex` calls `f` on
  each item in order and stops at the first it finds, as `position` does;
  `indexOf(x)` finds the first item `===` `x`, a float's `NaN` never.
- **Its test is `index !== -1`**, the one answer `LocalFacts::
  none_is_minus_one` gives, of a place found in a string too (ADR 0642).
- **An `Option` that leaves the function**, given back, passed on or
  held whole, keeps `$position`'s, `undefined` its `None`, as Rust's
  callers read it; so does a `position` of another iterator, a held one
  stepped before, an `iter_mut()`'s places, or a chain.

**The index found is below the slice's length till the slice changes**
(`body_queries/ranges/found.rs`): the range of the `Option` a slice's
`position` gives is its `Some`'s index's, from 0 to one below the length,
so `if let Some(index)`, a `match`'s `Some(index)`, a `let Some(index) = ..
else`, and a variable that holds the `Option` first bind `index` so. As
any bound by a length, a `push` or a `pop` moves it, and a change the walk
can't follow forgets it (ADR 0576). So `remove(index)` is `splice(index,
1)` and `insert(index, x)` `splice(index, 0, x)` (ADR 0576's amendment).

## Alternatives

- **`-1` as `None` everywhere**: a `position` given back would be read
  by Rust callers that ask `is_none()`, and by JS ones as a number.
- **`$position` then `?? -1`**: the helper still steps an
  `HTMLCollection`, and reads as neither JS nor Rust.
- **`indexOf` of identity tests only, `$position` of the rest**: two
  forms of one search; `findIndex` is JS's own of any test.

## Consequences

- React's test renderer's appendChild, insertBefore and removeChild are
  upstream's `indexOf`, `index !== -1` and `splice`, and the portal test
  meets `indexOf is not a function` as upstream does.
- Held by the corpus's `position_index_of`, against native Rust: identity
  and other tests, `match`, `if let`, `let .. else` and a held `Option`;
  floats with `NaN`; an `Option` given back; an iterator stepped first and
  an `iter_mut()`'s, kept; and a `pop()` between the search and `remove`,
  `$removeAt`, which panics as Rust's does. `rc_fn_ptr_eq` and
  `rc_fn_ptr_eq_of_moved` are `indexOf` now. Mutations
  `found-index-not-fresh`, `found-index-escaping`,
  `found-index-never-index-of` (`positions.ts`), `found-index-of-iter-mut`,
  `found-index-none-null`, `found-index-items-stepped`,
  `found-index-unbound`, `found-index-at-most-the-length`,
  `found-index-of-a-match-unbound`, `found-index-of-an-if-let-unbound`,
  `found-index-of-a-let-else-unbound` and `found-index-held-unknown`; and
  `tested-and-taken-returned`, `tested-and-taken-let-mention-unknown` and
  `tested-and-taken-other-variant-read`, #366's, the rule moved to
  `queries.rs`.
