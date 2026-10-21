# 0525. A cloned `map` or `filter` is made again

Status: Accepted. Extends [0522](0522-a-closure-cloned-apart-is-made-again.md)
and [0181](0181-iterator-clones.md); accepts part of the
[iterators draft](draft/cloned-and-buffering-iterators.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): a clone of an iterator
is where the iterator is, with a clone of its closure, as Rust's `Clone` of
a `Map` is; nothing reruns.

## Context

A lazy adapter is a JS iterator helper, `it.map(f)`, whose state hides in
the engine: a clone of one was refused ("cloning
`std::iter::Map<std::ops::Range<i32>, {closure}>`"). A `map` or a `filter`
holds nothing but its source and its function, though: a JS helper's
position is its source's. A clone is the helper made again of a clone of
each, as a closure cloned apart is (ADR 0522).

## Decision

**A `map` or a `filter` the crate clones is made by a function of its
source and its function, `makeMap`, and carries its clone,
`$cloning(it.map(f), () => makeMap(clone(it), clone(f)))`; `$clone(it)`
makes it again.**

```js
const makeMap = (it, f) =>
  $cloning(
    $iterator(it, $rangeNext).map((item) => f(item)),
    () => makeMap({ start: it.start, end: it.end }, $clone(f)),
  );
const it = makeMap({ start: 1, end: 4 }, counter);
const copy = $clone(it);
```

- **One owner of what's cloned**: `analysis::clones`, the types each call's
  `Clone` bounds are given, revealed through `impl Clone`, and what each
  clone clones in turn, its `Clone` impl's bounds, to a fixed point:
  `Map<I, F>: Clone` clones `I` and `F`. Closures read their part of it
  (ADR 0522).
- **One question, `iterator_cloned_apart`**: a `Map` or `Filter` of the
  cloned types, where a generic function's `Map<I, {closure}>` stands for
  each its callers clone; what's made and what's cloned both ask it.
- **Its source is cloned as it is**: an array's iterator's, a range's,
  another made again; a `copied()` is the iterator it copies (ADR 0480).
- **A closure that writes nothing it took is shared**: `clone_shares` asks
  what its captures do, not its kind, which a `map`'s bound makes `FnMut`.

Still refused, in [the limitations](../limitations.md): a clone of one that
keeps a count of its own, an `enumerate`'s, a `take`'s or a `skip`'s, whose
JS helper hides it; one of items with a destructor, or lent; and one
generic code makes where its source isn't bound by `Clone`, which it
couldn't clone.

## Why

`lazy_iterator_clone` clones a `map` of a closure that counts its calls,
partly stepped, a `filter` of a `map`, one through a `T: Clone`, one a
function gives as an `impl Iterator + Clone`, and one a derived `Clone`'s
field holds, against native Rust.

## Consequences

- `docs/limitations.md` loses "A clone of a lazy iterator", and gains its
  narrower remains.
- Mutations stop following impl bounds, revealing an `impl Clone`, or
  matching generic code's closure; share a source or a closure; and make
  every `map` cloned apart where a type parameter is cloned.

## Amendment: an `enumerate`, a `take` and a `skip`

Case: A ([0262](0262-when-rust-and-js-disagree.md)): what's given is the
same; a refusal is lifted.

**An adapter that holds its source and a count alone, an `Enumerate`, a
`Take` or a `Skip` (`recognition::source_and_count`), is cloned apart as
a `Map` is**: made by a function of its source and its count, which it
keeps as it steps and its clone is made at.

```js
const makeEnumerate = (it, count) =>
  $cloning(
    it.map((item) => {
      const index = count;
      count += 1;
      return [index, item];
    }),
    () => makeEnumerate($iter(it.items.slice(it.at, it.end)), count),
  );
```

`take(left)` counts `left` down as it gives each item, so its clone takes
what's left; `skip(n)` sets `n` to none once it gives an item, as it's
skipped them by then. A JS iterator helper's own count, `map`'s index or
`take`'s, can't be read, so each keeps its own where its clone can.
`test/corpus/cloned_counted_adapters.rs` clones each before and after it
steps, against native Rust; mutations `enumerate-apart-count-unkept`,
`take-left-unkept`, `skip-n-unkept`, `count-clone-from-start` and
`counted-adapter-not-apart`.
