# An iterator that's cloned, or holds items with a destructor, is rust-js's own

Status: Draft. Would extend [0465](../0465-dropping-an-adapter-is-closing-it.md),
whose adapters are closed to drop, [0485](../0485-an-adapter-holding-a-function-is-closed-to-drop-it.md),
[0429](../0429-a-double-ended-iterator-knows-its-back.md) and
[0493](../0493-a-zip-steps-from-its-back.md); and amend
[0164](../0164-double-ended-iterators.md), whose `rev()` of a lazy iterator
is refused, and [0181](../0181-iterator-clones.md), whose clone is
of an array's iterator alone.

Case: C ([0262](../0262-when-rust-and-js-disagree.md)): each closure runs
once, in Rust's order, and each item is dropped once, where Rust drops it.

## Context

A lazy iterator is a JS iterator: `items.values().map(f)`, stepped by
`next()`, closed by `return()` (ADR 0465). A JS iterator can't be cloned,
and holds what it has stepped where nothing can reach it. So rust-js
refuses:

- a clone of a lazy iterator, `it.clone()` of a `Map` or a `Filter`;
- `peekable`, `rev`, `flat_map` and `scan` of items with a destructor,
  which hold an item they've stepped, or step from the back;
- generic code given a `Peekable` of a type with a destructor.

Each is idiomatic Rust, and itertools, chrono and smallvec meet them. The
counted `Rc` (ADR 0320) met the same shape of problem: a representation
JS has, a value, serves most uses, and a few need one with more in it.
Its answer, a second representation chosen by an analysis, to a fixed
point, and the two never mixed, is the precedent.

## Decision

**An iterator the crate clones, or an adapter that holds items with a
destructor, is made of rust-js's own adapters, `$map(src, f)`, `$filter`,
`$peekable`, `$flatMap`, `$scan`, `$rev`: each an object with `next`,
`return`, and `clone`, over a source that knows its position, a slice's,
a range's, or a `Vec`'s own items. Every other lazy iterator is the JS
iterator it is today.**

```rust
let it = (1..4).map(|x| x * 2);
let v: Vec<_> = it.clone().collect();
println!("{v:?} {}", it.count());
```

```js
const it = $map($range(1, 4), (x) => (x * 2) | 0);
const v = it.clone().toArray();
console.log(`${$debug(v)} ${it.count()}`);
```

- **Which iterators are rust-js's own is one analysis's to say**,
  `iterators::is_own`, to a fixed point as `Counted` is: one that's
  cloned, one that `peek`s, `rev`s, `flat_map`s or `scan`s items with a
  destructor, one given to generic code that does, and every adapter
  below it, since its source must be clonable or closable too. A type
  parameter's iterator in a crate where any is rust-js's own makes every
  one so, as a generic `Rc` does (ADR 0494): the two must agree on what an
  iterator is.
- **A clone is Rust's `Clone`**: the source's position copied, a slice's
  index, a range's current, and each adapter's function and state cloned
  by the clone the crate's `Clone` gives; a `Vec`'s own items cloned by
  their `Clone`, as `vec::IntoIter::clone` does. A source with no clone, a
  closed-over generator, is refused at the clone, naming the source.
- **What an adapter holds is dropped when it's closed**: `$peekable`'s
  peeked item, `$flatMap`'s inner iterator, `$scan`'s state, `$rev`'s
  buffered back, each by the drop its item's type has, given as ADR 0489
  gives one, in the order Rust's `Drop` runs: the inner then the outer.
- **`$rev` steps from its source's back** as `$zipBack` does (ADR 0493),
  of a source that knows it, a slice's or a `Vec`'s own items; of a
  `map` whose closure would run in another order, it's refused still
  (ADR 0164), the one case no representation fixes.
- **Across the crate boundary**, a library's generic iterator is its
  consumer's: the manifest says which of the library's iterators are its
  own, as [the counted `Rc` draft](counted-rcs-across-crates.md) says
  which `Rc`s it counts.

## Why

- **It's std's**: a corpus case clones a map, a filter and a chain of a
  range and of a `Vec`, steps both, counts both, and drops a peeked item
  with a destructor, a reversed `Vec`'s, a flat-mapped inner, each with
  native Rust; `itertools`' `tee`, `multipeek` and `rev` compile.
- **It's one representation per iterator**: the analysis answers, and
  nothing mixes the two, as nothing mixes a counted `Rc` with a plain one.
- **The JS a person writes stays**: every iterator nobody clones or
  buffers is `items.values().map(f)`, as today.

## Alternatives

- **Materialize on clone**, `[...it]` then iterate twice: runs each
  closure before Rust would, and once where Rust runs it twice. Case C,
  not allowed.
- **Re-run the chain from its start**, a clone as a factory call: a clone
  copies the current position, not the first; and the source's items,
  a `Vec`'s with a destructor, are moved, not re-readable.
- **Every lazy iterator rust-js's own**: one representation, no analysis,
  and every `map` a helper call. Against the output promise, for a case
  most programs never meet.

## Consequences

- **Runtime helpers**: six adapters with `clone` and `return`, each a
  file in `src/runtime/`, held by mutations that lose a clone's position,
  drop a peeked item twice, or close an inner iterator late.
- **The limitations page loses four lines**: a clone of a lazy iterator;
  `peekable`, `rev`, `flat_map` or `scan` of items with a destructor;
  generic code given a `Peekable` of one; and gains one: `rev()` of a
  `map` whose closure would run in another order.
- **ADR 0181 is subsumed**: an array's iterator's clone becomes one case
  of this.
