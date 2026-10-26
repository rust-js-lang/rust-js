# 0925. A chain's `nth` drops what `b` passes over, and a compiled struct drops what it holds itself, never closed again

Status: Accepted, under the user's goal of 2026-10-26 ("keep going until
all done, the choice is your recommendation"). The plan "rust-js: one
representation per iterator", its third phase, from #339's survivors.
Extends [0913](0913-std-s-generic-try-is-a-dictionary.md) and
[0561](0561-std-bodies.md); amends
[0542](0542-a-held-adapter-keeps-its-source-and-state.md), whose counted
adapters are std's structs now.

Case: C ([0262](0262-when-rust-and-js-disagree.md)): results change.
`Count(100, 103).chain(Count(110, 112)).nth(4)` of the crate's iterator
of items with a destructor never dropped `111`, which Rust's `Chain::nth`
drops as `b` passes it.

## Context

#339 listed the iterators' mutations that survive on main. Each was a path
held adapters took before they were std's structs (ADRs 0871, 0913, 0920):

- `$lazyChain`'s `nth`, a chain made in place, advanced `a` by the items'
  drop and then asked `b.nth(n)` given none, so `b`'s std default `nth`,
  stepping the crate's iterator, passed its items undropped.
- An explicit `Enumerate`, `Take` or `Skip` (ADR 0542), of a kind a struct
  holds or the crate clones, is none now: each is `HELD_COMPILED`, which
  `explicit_adapters` leaves out, so their `Debug`, `Clone` and back arms,
  and what added cloned ones to the set, ran for nothing.
- A compiled struct, a `fuse()` or a `peekable()`, drops what it holds by
  its own `return` and at its end; `closing_drop` wrapped it in
  `$closing(it, (it) => { it.return(); })` too, and `rev()` closed what it
  steps again, each a second close of it.
- `$closing`'s `!it.$closed` and `$lazyChain`'s default item drop each
  repeated what the struct's own `return`, or every caller, already does;
  a `count` of what `nths_a_slice` is a compiled `count` now (ADR 0913).

## Decision

- **A chain's `nth` gives `b`'s `nth` the items' drop, as `a`'s advance is
  given it**: what either side passes over is dropped as Rust drops it.
- **A compiled std struct drops what it holds itself**: what holds one
  closes it as it is, never wrapped in `$closing` to close it again
  (`closing_drop`), nor `rev()`'s source closed twice.
- **The paths no adapter takes are gone**, each with its mutation: the
  explicit counted adapters' `Debug`, `Clone`, back and `held_source`; the
  count-by-slice disjunct; `$closing`'s second `$closed` guard;
  `$lazyChain`'s default drop; and `$iteratorValue`, which only the
  counted adapters' `Debug` and `Clone` asked for.

## Consequences

- `chain_nth_drops_what_it_passes.rs` runs as native Rust does, as does
  `own_nth_skipped_in_place.rs`, an in-place `skip()` or `take()` of the
  crate's iterator with its own `nth`, asked as std asks it.
- Seven cases' JS loses its `$closing(fuse, (it) => { it.return(); })`
  wrappers: 262 lines out, 138 in (`own_iterator_dropped_through_adapters.js`,
  `own_fold_through_adapters.js`, `chain_sides_own_steps.js`).
- Of #339's iterators' survivors: `chain-nth-given-drop-unused`,
  `nth-dropping-drop-ungiven`, `skipping-length-its-source`,
  `skipping-nth-unasked`, `take-own-nth-needs-item-drop`,
  `rev-own-impl-in-place` and `cloned-apart-unbound-shared` are caught by
  a case that still reaches them; `adapter-dictionary-own-unpassed` and
  `rev-of-closed-chain-unturned` change how the JS reads alone, so are
  `snapshots: true`; `closing-fold-taken-closed-again`,
  `chain-side-advanced-undropped`, `lent-struct-field-hidden` (a field is
  `count$`, never a helper's name, ADR 0912), `take-debug-as-enumerate`,
  `counted-clone-from-start`, `rev-source-unclosed`, `slice-count-folded`
  and `compiled-std-source-unclosed` went with their code, as did
  `cloned-counted-not-explicit` and `held-source-range-unknowing`.
- Mutations: `chain-nth-b-undropped`; `closing-twice` and
  `crate-iterator-unclosed` follow their code.
