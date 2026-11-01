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

## Indexed lists (amended, rd274)

A `NodeList` and the DOM's other indexed lists are stepped by their
binding's `Iterator.from(list)`, `node_list::iter(list)`. Upstream counts
their indices too, `for (let j = 0; j < appearingElements.length; j++)`,
as React DOM's Fizz instruction set does, whose ES5 build polyfilled each
`for…of` of them.

- **The fact:** WebIDL gives such a list an indexed getter, `getter T
  item(unsigned long index)`, and an integer `length`, its own or its
  parent's, and makes its `[Symbol.iterator]` `%Array.prototype.values%`,
  unless it iterates pairs, a map or a set. The webapi generator reads it
  from WebIDL (`indexedList`) and marks each such struct
  `#[rust_js::indexed]`: 37 of them, `NodeList`, `HTMLCollection`,
  `DOMTokenList`, `HTMLSelectElement`, `CSSStyleDeclaration` and the rest.
  The attribute is refused of anything but a struct.
- **The loop:** a binding's call of `Iterator.from(list)` given an indexed
  list (`bindings::is_indexed_list` of its first parameter) is said to
  step an array's items, `Known::ArrayItems`, and its `for` is the same
  index loop, under the same conditions: a variable the body doesn't
  assign.
- **Equivalence, live lists too:** `Array.prototype.values` reads the
  list's `length`, then `list[i]`, at each step, as `i < list.length` and
  `list[i]` do. A live list, `childNodes` or `children`, that the body
  shrinks, `element.remove()`, is stepped the same by both: each skips the
  item that took the removed one's index. `Iterator.from` of the list's
  own iterator gives it as it is, and an array iterator's `return`, which
  a `break` calls, does nothing. Case N of ADR 0262: nothing observable
  changes. Held by `test/lowering-index-loops.test.ts`, which runs both
  forms over happy-dom's lists and over a list built as WebIDL specifies
  one, read live through a getter.

## A `Vec`'s `extend` of an indexed list (amended, the user's decision, 2026-11-01)

The Fizz instruction set's `suspensey_images.extend(node_list::iter(appearing_images))`
printed `$extend(suspenseyImages, Iterator.from(appearingImages))`, and
Closure's ES5 build polyfilled `Iterator.from` and its iteration, about
3.6 KB. Upstream pushes each image itself.

- **A `Vec`'s `extend` of an indexed list is an index loop that pushes
  each item:** `for (let i = 0; i < list.length; i++) { v.push(list[i]);
  }`, of a `NodeList`'s `Iterator.from(list)` or of std's iterator over an
  array's items made where it's extended, `v.extend(xs.iter())`: what
  `Known::ArrayItems` says where the iterator is made. A list that isn't a
  variable is held first, `const items = made(2);`, as is a collection
  that isn't one; a `&T` pushed onto `T`s is copied in the loop, `{
  ...points[i] }`, as `copied_into` would copy it.
- **Not yet of a slice or a `Vec` given whole,** `v.extend(&xs)`, which
  stays `$extend(v, xs)`: a trait's dictionary, `Vec<T>: Extend<A>`'s
  generic `extend`, is lowered once of the type it's first made with and
  given any iterable after (`test/corpus/generic_extend.rs`, whose range
  an index loop of its first array's type read as `length` 0). The type
  where `extend` is called can't say it's an array there; a fact where the
  array is made would.
- **Not `push.apply(v, list)` or `v.push(...list)`**, which upstream writes
  in places: they pass each item as an argument, and throw a `RangeError`
  past the engine's limit of a call's arguments, where Rust's `extend`
  never fails. The user chose correctness. The loop pushes as many as the
  list has; `test/lowering-index-loops.test.ts` extends by 300,000.
- **Equivalence:** the same as the `for`'s. `$extend` stepped the list's
  own iterator, which reads `length` and `list[i]` at each step, as the
  loop does; pushing onto the `Vec` changes no `NodeList`, and Rust lets
  no list be the `Vec` it extends (`extend_from_within` is another
  function). Case N of ADR 0262. Native comparison:
  `test/corpus/vec_extend_indexed.rs`.

## What it doesn't do yet

- `enumerate()`, `for (const [i, x] of v.entries())`, stays as it is.

## Consequences

- Most corpus programs with a loop over an array changed, and the guide's
  examples with them. Native comparisons are unchanged.
- `format.rs` keeps a mapping only on its node's line, where the formatter
  wrote less of that line than the code. A loop's binding, which has no
  span, put one past the text's end. The order of the mappings it moves,
  which no program it prints puts out of order now, is held by
  `format::tests::moved_tokens_in_order`.
- A loop over a `RefCell`'s borrow is a `for` again, `for (let i = 0; i <
  borrow.length; i++)` in its `try`: a step whose temporary was named for
  the pattern that binds it, `let item; try { item = $next(iter); } ..`
  (ADR 0569's #444), is the `for`'s own binding (`tried::binding`), where
  it had printed `while ((item = $next(iter)) != null)` since #444.
- Held by `test/corpus/array_counted_loops.rs`, run against native Rust,
  and the mutations of `scripts/mutations/lower/mir/tidy/loops.ts` and
  `lower/mir/iter/sources.ts`.
