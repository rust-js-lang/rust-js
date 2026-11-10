# 0464. A pick drops what it passes over, and a consumed iterator is dropped after

Status: Accepted. Amends [0190](0190-generic-no-destructor.md); extends
[0098](0098-destructors.md), [0421](0421-std-associated-type-drops-are-given.md)
and [0057](0057-ordering.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's dropped, when,
and what a comparison is given, as std's.

## Context

itertools picks the greatest of items it's given, `iterable.into_iter()
.max()`, in generic code, of items its caller may give a destructor, and
`max_by`, `min_by` and `max_by_key` of its own. Each was refused of items
with a destructor, as one that may leave some undropped: std's fold drops
each it passes over as it goes. And generic code that consumed an iterator
its caller gave, `items.fold(..)` or `last()`, took no destructor of it
(ADR 0190), as JS doesn't drop the iterator once it's done, where Rust does,
a crate's iterator's own `Drop` running then.

`$maxBy` wasn't std's either: it called `compare(item, kept)`, where std's
`cmp::max_by` calls `compare(kept, item)`, and `max_by_key`'s key was made
again for each comparison, where std's is made once for each item. A
comparison or a key that prints, or counts, showed it.

## Decision

**`max`, `min`, `max_by`, `min_by`, `max_by_key` and `min_by_key` are std's
fold, `compare(kept, item)` as each comes, a key made once for each, and
each passed over dropped as it's passed:**

```js
$maxBy(items, by_name, false, loudDrop_drop)
$minByKey(items, rank, $cmp, false, loudDrop_drop)
```

- **`$maxBy` and `$minBy` are one fold, `$extremeBy`**: the later of two
  equal items kept by `max`, the earlier by `min`, as std's; boxed where an
  item may look like `None` (ADR 0051), and given the items' drop where they
  have a destructor. `$maxByKey` and `$minByKey` pair each item with its key
  as it comes, as std's do. A key with a destructor is refused.
- **They leave none undropped**, `IterComb::leaves_none`, as `partition`
  and `filter_map` move each on.
- **An iterator consumed by value is dropped after, by the drop its type
  has**: the crate's own iterator's, as before, and now a generic one's its
  caller gives (ADR 0421), `dropIIntoIter?.(iter)` once `last()` or `fold`
  is done. Exhausted, a `Vec`'s drops nothing more, and a crate's iterator
  runs its own `Drop`, where Rust does.

```js
function final_one(items, dropIItem, dropIIntoIter) {
  const iter = Iterator.from(items);
  const result = $lastDropping(iter, dropIItem);
  dropIIntoIter?.(iter);
  return result;
}
```

## Why

- **It's std's, run for run**: `extreme_drops` compares each pick's drops,
  each comparison's arguments and each key's call with native Rust, in
  concrete code and in generic code given a crate's iterator with a
  destructor of its own.
- **ADR 0190 shrinks to what's still unsound**: a lazy adapter of a generic
  iterator that it gives back, `it.map(f)`, which drops it with the adapter.

## Consequences

- **What generic code's callers may give grows**: a consumed iterator of
  items with a destructor, `fold`, `count`, `last`, `max`, as itertools'
  free functions take.

## Amendment: `max_by`'s pick is MIR's (2026-11-06, #799)

Since c4ff618a6, `max_by` and `min_by`, by a key or not, are std's
iterator methods from MIR (ADR 0470, amended). The recognizers'
`IterComb::MaxBy` no longer reaches what says it leaves no item undropped
(`leaves_none`), nor do it and `MaxByKey` reach what says their `Option`
is boxed as it's made (`refuses_boxed_option`), in any case or test, each
probed: those arms are gone, with `max-by-leaves-drops` and
`boxed-max-by-refused`. Where
one were reached, it'd be refused, never wrong.

## Amendment: a `_key` form's key is what its function gives (2026-11-10, #1223)

`max_by_key`, `min_by_key` and `is_sorted_by_key` read their key's type
from the function's signature, a closure's or `fn_sig`'s, which a
`Box<dyn FnMut>`, a `&mut F` or any other `FnMut` but those hasn't:
rust-js crashed. The key is what std's bound names it,
`<F as FnOnce<(&Self::Item,)>>::Output` (`collect.rs`' `key_of`),
normalized in the body's typing environment, whatever `F` is, so its
drop is asked of the type it is. Found by #1204's `dyn-fn` rewrite of
eight corpus cases. Corpus case `key_functions_boxed`, mutation
`key-of-a-boxed-function-by-its-signature`. A key with a destructor kept
past a function with one is dropped after the function in native Rust and
before it in JS, closures too: #1231, filed.
