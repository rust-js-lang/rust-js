# 0995. A std body takes a JS iterator's dictionary where it's given none

Status: Accepted. Phase 3's first step of the plan "one representation per
iterator": less of what std's compiled structs write at each use.

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable. A call
leaves out what its callee would take anyway.

## Context

A std body compiled into `@rust-js/std` takes a dictionary for each of its
bounds. Of a JS iterator, the one most calls give, it's the same every
time: `{ next: $next }` of an `Iterator` bound, `{ next_back: $nextBack }`
of a `DoubleEndedIterator` one. Each call wrote each out:

```js
mapDoubleEndedIterator_rfold(iterator, init, f, { next: $next }, { next_back: $nextBack })
```

In the `--std-iterators` build, about 2,500 of these, 1,400 of them alone
on a line, were much of what made it longer than the normal build.

## Decision

`default_dictionary`, of `evidence.rs`, is the one rule: a std body's
`Iterator` and `DoubleEndedIterator` bounds take that dictionary where
they're given none.
- `evidence_params` declares each such parameter defaulted:
  `AIterator = { next: $next }`.
- `evidence_args_given` leaves out the arguments at a call's end that are
  that dictionary, where no drop is given after them.

A dictionary entry that then only forwards its arguments is the function
itself: `next_back: mapDoubleEndedIterator_nextBack`.

The walks that collect what a module reads (`Item::visit_vars`) and the
helpers it asks for (`each_item_expr_mut`) now visit parameters'
defaults. Before, they never had to read anything there.

## Consequences

| Build | Before | After |
|---|---|---|
| `--std-iterators` | 106,594 lines | 105,649 |
| Normal | 94,450 lines | 94,354 |
| Gap | 12.9% | 12.0% |

The normal build's change is 41 snapshots. Both corpora run as native Rust
does. The mutation is `default-dictionaries-kept`; `closure-reads-unseen`
covers an arrow's defaults.
