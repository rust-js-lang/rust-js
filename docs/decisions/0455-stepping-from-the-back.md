# 0455. `nth_back`, `rfold` and `rfind` step from the back

Status: Accepted. Extends [0429](0429-a-double-ended-iterator-knows-its-back.md)
and [0098](0098-destructors.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)): the same items, in
the same order, dropped where std drops them.

## Context

A double-ended iterator, an array's, a range's, the crate's own or a
generic one, is a JS iterator that knows its back, which `next_back`
steps, `$nextBack(it)` (ADR 0429). But `nth_back`, `rfold` and `rfind`,
which either's `Either` and itertools forward to what they hold, were
refused, of any iterator.

## Decision

**`it.nth_back(n)`, `it.rfold(init, f)` and `it.rfind(p)` are
`$nthBack(it, n)`, `$foldBack(it, init, f)` and `$findBack(it, p)`, which
step `it` from its back as `next_back` does**: the iterator itself where
it has one, as `next_back`'s, and one made as a double-ended iterator is
otherwise, a range's of its bounds.

1. **Of items that could look like `None`**, `nth_back`'s and `rfind`'s
   found item is its `Some`, `$some`, given to the helper (ADR 0051).
2. **Of items with a destructor**, what `nth_back` and `rfind` pass over
   is dropped as it's passed, the item's drop given to the helper, as
   std's are; `rfold` gives each to its function, which drops it. A
   generic one's item of an associated type, which has no drop to give,
   is refused, as forward `nth` is.
3. **`find_map` of items with a destructor**, which gives each to its
   function, isn't refused, as `filter_map`'s isn't (ADR 0098): one
   answer, `moves_each_on`, which recognition asks of a method's name too.

## Why

- **It's std's iterator**: either and itertools forward each method to
  what they hold, and code that walks a list from its end calls them.
- **One way to step from the back**: `it.back()`, which `next_back`
  steps, the same for each kind of iterator.
- **It's tested**: corpus cases step a generic iterator, a range, an
  array's, through a `&mut` and by value, of numbers, of `Option`s, and
  of items with a destructor, against native Rust; a mutation of each
  part is caught.
