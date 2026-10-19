# 0472. An `Option` an `Option` holds is given as it's held

Status: Accepted. Extends [0051](0051-generic-options.md),
[0464](0464-a-pick-drops-what-it-passes-over.md) and
[0071](0071-stepping-iterators.md).

Case: A ([0262](0262-when-rust-and-js-disagree.md)): the same values, as
the representation they're held in.

## Context

An `Option` of what could look like `None`, a type parameter's or an
`Option`'s, holds its `Some` boxed (ADR 0051), and a call that gives one
was refused unless it was known to make it so. itertools takes the inner
`Option` of an `Option<Option<I::Item>>`, `a_cur.unwrap_or_else(|| a.next())`,
and collects the pick of `min_by`, `iter.min_by(f).into_iter().collect()`,
each refused. And std's iterator of another's items, `option::IntoIter
<I::Item>`, was left the array `into_iter()` makes, which `collect` stepped
as an iterator, and threw.

## Decision

- **What a `Some` or an `Ok` holds is given as it's held**: `unwrap`,
  `unwrap_or`, `unwrap_or_else`, `unwrap_or_default` and `unwrap_unchecked`,
  and a `Result`'s, of an `Option` of an `Option`, give the inner one, box
  and all, as `flatten` does.
- **`max_by`, `min_by` and their `_by_key` forms give their pick boxed**, as
  `$maxBy` makes it (ADR 0464), as `max` and `min` do.
- **A std call's iterator is std's of whatever items**, `option::IntoIter<
  I::Item>`, made a JS iterator as the call's value, as one of a type's items
  is.
- **The runtime's generators are JS iterators as they're made**:
  `$lazyChain(a, b)`, `$lazyZip`, `$lazyTakeWhile` and the rest are written
  with no `Iterator.from` around them. The compiler's list of them,
  `ITERATOR_HELPERS`, is held to the runtime's `function*`s by a test.

```js
function first_or(cur, it) {
  return cur != null ? $someValue(cur) : $nextSome(it);
}

const chained = $lazyChain($closing(value, drop), $closing(Iterator.from(tmp), drop));
```

## Why

- **It's the same value**: `boxed_option_unwraps` runs each, of a `None`
  inside a `Some` and of a `Some` of a `Some`, with native Rust.
- **One place says what's a JS iterator**, `is_js_iterator`, of one list
  held to the runtime.

## Consequences

- **What compiles grows**: the unwraps of an `Option` of an `Option` of a
  type parameter, and `max_by`'s pick of one.
- **Less noise**: `Iterator.from` is gone from around each lazy adapter's
  generator.
