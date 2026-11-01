# 1042. A loop over an array counts its indices: `for (let i = 0; i < v.length; i++)`

Status: Accepted (the user's decision, 2026-11-01). Extends
[0025](0025-vec-loops-refcell-mut.md), which made a `for` over a `Vec`
a `for…of`, and [0904](0904-a-loop-from-the-end-counts-down.md), whose
loop over an array's items from the end already counts down its indices.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference. An array's iterator, which `for…of` steps, reads the array's
`length` and then its item at each step, and the loop does the same: `i <
v.length`, then `v[i]`. Both see the same items, live, whatever changes
the array meanwhile. Rust doesn't let anything change it: the loop borrows
it or owns it.

## Context

React DOM's Fizz instruction set is shipped as inline scripts. Closure
compiles them to ES5, and each `for…of` brings about 2 KB of `Symbol` and
iterator polyfills with it. Upstream writes its loops over arrays as
counted ones:

```js
for (let j = 0; j < appearingElements.length; j++) {
  const appearingElement = appearingElements[j];
```

The port's `for &suspensey_image in &suspensey_images` printed `for (const
suspenseyImage of suspenseyImages)`: different JS, and m6-wife's rd274
build paid for the polyfills.

## Decision

**A `for` over std's iterator of an array's items is an index loop**:
`for (let i = 0; i < v.length; i++) { const x = v[i]; .. }`, a pattern's
`const [a, b] = v[i];`.

- **Which iterators:** a slice's `Iter` (`for x in &v`, `v.iter()`), and
  a `Vec`'s or an array's `IntoIter` (`for x in v`), whose JS value is the
  array itself (`Recognition::steps_its_array`). Lowering says so on the
  `$iter` it makes, `Known::ArrayItems`, which `stepped_slices` reads too.
- **Which arrays:** a variable that the body doesn't assign, so `v` reads
  the same array at each step. A `Vec` the loop owns may be given another
  value in its body, `v = vec![x]`, which `for…of` never steps. Anything
  else stays a `for…of`.
- **What stays a `for…of`:** a map's or a set's items (their JS value may
  be the map), a string's `chars()` (JS's `s[i]` is a UTF-16 unit, not a
  char), and an adapter's (`copied()`, `map(..)`).
- **The counter's name:** `i`, as a person writes it, and the next letter,
  `j`, then `k`, for each loop inside one. Sibling loops each have their
  own `i`, which is scoped to its loop. Where the function already uses
  the name, the next one is taken.

## Why

- **It's upstream's JS**: React DOM writes its loops over arrays this way,
  and the Fizz runtime's ES5 build carries no iterator polyfills for them.
- **One form for one construct**: a loop over an array's items counts its
  indices, from the start as from the end (ADR 0904).

## What it doesn't do yet

- A live `NodeList` (`node_list::iter(list)`) and a `js::Array` are
  stepped by their binding's `Iterator.from(..)`. Saying one is an indexed
  list needs a fact about the binding, WebIDL's indexed getter with a
  `length`, and those loops stay `for…of` until bindings carry it. That
  is the next step of this decision.
- `enumerate()`, `for (const [i, x] of v.entries())`, stays as it is.

## Consequences

- Most corpus programs with a loop over an array changed, and the guide's
  examples with them. Native comparisons are unchanged.
- `format.rs` keeps a mapping only on its node's line, where the formatter
  wrote less of that line than the code. A loop's binding, which has no
  span, put one past the text's end.
- Held by `test/corpus/array_counted_loops.rs`, run against native Rust,
  and the mutations of `scripts/mutations/lower/mir/tidy/loops.ts` and
  `lower/mir/iter/sources.ts`.
