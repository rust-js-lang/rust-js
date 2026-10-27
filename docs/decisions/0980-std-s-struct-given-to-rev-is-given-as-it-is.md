# 0980. std's struct given to std's `rev()` is given as it is

Status: Accepted, under the user's goal of 2026-10-26. Amends
[0871](0871-rev-is-stds.md): a value given as it is to a std body that
steps it from its back is made knowing its back only where its dictionary
is a JS iterator's, `{ next_back: $nextBack }`.

Case ([0262](0262-when-rust-and-js-disagree.md)): none, no observable
difference. The compiled body steps the value by the dictionary it's
given, which for std's struct is its compiled back.

## Context

A value given as it is to a std body compiled that steps it from its
back, `rev()`'s `self`, was made knowing its back where it was given:
`$knowing(value, nextBack, len, .., tryRfold)`. That's what a
recognizers' adapter needs, a JS iterator whose dictionary's `next_back`
is `$nextBack`, which steps the value's own back.

std's compiled struct got the same, though its `DoubleEndedIterator`
dictionary is its compiled back methods. Every `rev()` of a held
`Chain`, `Map` or `Copied` wrote those methods twice: in `$knowing`, and in
the dictionary given beside it. With std's structs alone `rev_chain` was
521 lines.

## Decision

A value given as it is to a std body compiled is made knowing its back
only where it isn't std's struct compiled there (`compiled_struct_at`).
std's struct is given as it is, and its dictionary steps it.

## Consequences

`rev_chain` with std's structs is 373 lines. In the normal build,
`std_map`'s held `Map` given to `rev()` loses its `$knowing` too. The
mutation is `compiled-struct-made-knowing`.
