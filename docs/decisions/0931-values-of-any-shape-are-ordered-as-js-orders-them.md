# 0931. Values of any shape are ordered as JS's `<` and `>` order them

Status: Accepted, the user's choice of 2026-10-26. Extends
[0612](0612-unknown-equality-is-strict.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a value of any
shape has no meaning but JS's, so its order is JS's, where Rust's
contract asks; `<=` is Rust's, paid for with `$partialCmp`.

## Context

trekhleb/javascript-algorithms' `Comparator`, which its search, sorting,
heap, linked list and trees use, orders values of any shape:

```js
static defaultCompareFunction(a, b) {
  if (a === b) {
    return 0;
  }
  return a < b ? -1 : 1;
}
```

Its own test compares numbers and strings with one instance. A port
generic over `T: PartialOrd` makes a JS caller pass a dictionary, which
upstream's API and tests don't; `js::Unknown`, a value of any shape, had
`==`, JS's `===` (ADR 0612), and no order.

## Decision

**`impl PartialOrd for js::Unknown`: `Less` where `a < b`, `Greater` where
`a > b`, `Equal` where `a === b`, else none**, `NaN`'s, or an object's
and another's. In JS:

| Rust | JS |
|---|---|
| `a < b`, `a > b` | `a < b`, `a > b` |
| `a.partial_cmp(b)` | `$partialCmp(a, b)`, `-1`, `1`, `0` or `undefined` |
| `a <= b`, `a >= b` | `$partialCmp(a, b) <= 0`, `>= 0` |

So `if a == b { 0 } else if a < b { -1 } else { 1 }` is upstream's
`defaultCompareFunction`.

## Why

- **`Equal` is `===`'s, as `==` is**: Rust's contract has `partial_cmp`
  `Equal` exactly where `==` holds. So `<=` can't be JS's `<=`, which
  coerces, `"1" <= 1` and `{} <= {}` true where `===` isn't: it's `<` or
  `===`, which `$partialCmp`, the float's, already is.
- **`<` and `>` are JS's own**: each is `Less` or `Greater` exactly where
  JS says so.

## Alternatives

- **Generic ports, JS passing a dictionary**: upstream's API and tests
  change.
- **Numbers only**: Comparator's own test, and any port of strings or
  objects, couldn't be written.

## What holds it

`lowering-bindings.test.ts`'s "js::Unknown's < is JS's, its partial_cmp
$partialCmp", run over numbers, strings, `NaN` and objects, and the
mutations `unknown-lt-by-partial-cmp`, `unknown-le-as-js` and
`unknown-unordered`.
