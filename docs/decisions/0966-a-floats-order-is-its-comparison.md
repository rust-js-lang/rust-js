# 0966. A float's `partial_cmp` against an `Ordering` is its comparison, and `contains` of floats `indexOf`

Status: Accepted. Extends [0371](0371-tests-and-names-as-written.md)'s turned
comparisons and [0053](0053-partial-eq.md)'s `contains` to what
React's test selectors write.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. Each form is the value
of the other, for numbers, where comparing runs nothing.

## Context

React's `findBoundingRects` merges adjacent rects:

```js
!(otherBottom < targetTop) && !(otherTop > targetBottom)
```

whose port, `other_bottom.partial_cmp(&target_top) != Some(Ordering::Less)`,
was `$partialCmp(otherBottom, targetTop) !== -1`. And `observeVisibleRects`
asks `nextInstanceRoots.indexOf(target) < 0`, whose port's `!next.contains(target)`
of values of any shape, or of floats, was `!next.some((each) => each ===
target)`.

## Decision

- **`$partialCmp(a, b)` compared with an `Ordering`, `=== -1`, `=== 1` or
  `=== 0`, is `a < b`, `a > b` or `a === b`, and `!==` their negations
  `!(a < b)`, `!(a > b)`, `a !== b`**, where `a` and `b` are floats or
  number literals (tidy's `ordered_floats`, of the names whose locals are
  floats, `Hints::floats`): `$partialCmp` is `a < b ? -1 : a > b ? 1 : a
  === b ? 0 : undefined`, so each test is its value; `NaN` is none of them,
  and `-0` equals `0`, in both. Not of values of any shape ([0931](0931-values-of-any-shape-are-ordered-as-js-orders-them.md)),
  whose `valueOf` the helper may run twice where one comparison runs it
  once.
- **`contains` of items `===` compares, a float's or a value's of any
  shape, is `indexOf(x) >= 0`**, which finds no `NaN`, as Rust's `==`
  doesn't, where `includes` would; and `!(xs.indexOf(x) >= 0)` is
  `xs.indexOf(x) < 0`, as an `indexOf` is never `NaN` (`turned`, which
  takes a `length` so already).

## What holds it

- `test/corpus/float_partial_cmp_tests.rs`, every `Ordering` test of
  floats with `NaN`, `-0.0` and infinities, and `contains` of `NaN`,
  against native Rust; `test/lowering-bindings.test.ts`, a value of any
  shape's stays `$partialCmp(a, b) !== -1`.
- Mutations: `ordered-floats-of-any-shape`, `ordered-floats-equal-turned`,
  `index-of-may-be-nan` (`tidy/branches.ts`), `ordered-floats-unread`
  (`tidy/stages.ts`), `contains-strict-some`, `contains-strict-includes`
  (`combinators.ts`).

## Not decided here

`position(|h| Rc::ptr_eq(h, x))` then `remove(index)` stays `$position`
and `$removeAt`: upstream's `indexOf` gives `-1` where `position` gives
`None`, `undefined`, and `$position`'s subject may be an iterator, not an
array. A pair read from an array index, `stack[index][0]` and
`stack[index][1]`, where a `let (a, b) = stack[index].clone()` is
destructured elsewhere, is left too.
