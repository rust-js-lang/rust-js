# 0984. A map or a filter cloned apart keeps its back

Status: Accepted. Amends [0525](0525-a-cloned-map-is-made-again.md): a
`map` or a `filter` cloned apart and stepped from its back is made as one
not cloned apart is, `$mapEnds` or `$filterEnds` (ADRs
[0455](0455-stepping-from-the-back.md),
[0526](0526-a-filter-steps-from-its-back.md)). Found by #382's triage.

Case ([0262](0262-when-rust-and-js-disagree.md)): results change. The
program threw where Rust steps the filter from its back.

## Context

```rust
let mut counted = v.iter().enumerate().filter(|(i, _)| *i != 1);
let again = counted.clone();
println!("{:?} {:?}", counted.next(), counted.next_back());
```

A held `filter` that's cloned is cloned apart: made by a function of its
source and its predicate, `makeFilter`, that its clone calls again with a
clone of each. Its maker wrote JS's `it.filter(f)`, which has no back:
`$nextBack(counted)` threw `TypeError: it.back is not a function`, where
Rust gives `Some((4, 50))`. Not cloned, the same filter is
`$filterEnds(..)`.

## Decision

A `map` or a `filter` cloned apart that the crate steps from its back
(`stepped_from_its_back`) is made of its source made knowing its back
(`as_double_ended`): `$mapEnds(source, f)` or `$filterEnds(source, f)`,
as one not cloned apart is. One stepped only from its front is JS's
`it.map(f)` or `it.filter(f)`, as before.

## Consequences

`test/corpus/counted_adapters_given.rs` checks a `take()` and an
`enumerate()` given to generic code, and held filters of them stepped
from both ends and cloned, against native Rust. The mutation is
`cloned-apart-filter-unknowing`. The case also catches #382's
`taking-front-uncounted`.
