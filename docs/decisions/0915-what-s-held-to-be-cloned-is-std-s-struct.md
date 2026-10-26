# 0915. What's held to be cloned is std's struct

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its second cause: one iterator in two
forms. Fixes #308. Extends [0871](0871-rev-is-stds.md),
[0525](0525-a-cloned-map-is-made-again.md) and
[0913](0913-std-s-generic-try-is-a-dictionary.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change. A
cycled `(0..n - 1).chain((1..n).rev())` threw in `revClone_clone`, and
`(1..4u8).rev().cycle()` was refused.

## Context

A `rev()`, a `skip()` or another adapter std compiles is made in place,
the recognizers' JS iterator, where what it's given to steps it as one.
`cycle()` and an adapter cloned apart (ADR 0525), a `chain()` a
`cycle()` clones, hold what they're given and clone it, and the clone of
a `rev()` is std's compiled `Clone`, `revClone_clone`, of std's struct.
So the chain's maker was given a JS iterator and cloned it as a struct.
On main the same held for a zip's `skip()` (`cloned_chains_and_cycles.js`),
whose clone was `skipClone_clone` of `[5, 6].values().drop(1)`.

## Decision

**What `cycle()` or an adapter cloned apart holds to clone is std's
struct**, which its compiled `Clone` clones: an adapter std compiles given
to either isn't made in place (`in_place_adapters`, of the crate's cloned
types, `cloned_types`, read when the bodies are collected). One that's
cloned apart itself, a `map()`'s, stays made by its own maker, as before.
`cycle()` of std's struct steps it as the JS iterator it is.

## Consequences

- `cycled_reversed_ranges.rs` runs as native Rust does: a cycled zigzag,
  a cycled `rev()`, and a cycled `skip()`.
- `cloned_chains_and_cycles.js`: the zip's `skip()` is
  `iterator_skip($iter([5, 6]), 1, ..)`, the struct its clone clones.
- The readability baseline is blessed: `$iter` 85.2 to 85.3 in 10,000
  lines, the new case's `$iter([5, 6, 7])` and the zip's `skip()`'s
  source; `Iterator.from` 15.2 to 15.1.
- Mutations: `cycle-source-in-place`, `cloned-apart-part-in-place`,
  `cloned-apart-part-held`, `cycled-std-struct-refused`.
