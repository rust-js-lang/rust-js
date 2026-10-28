# 1004. The mutations surviving at main's head

Status: Accepted. Part of #382.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. No JS a program
gets changes. This changes what the mutations check, and removes code
nothing reaches.

## Context

Main's nightly failed: 14 mutations survived at its head, and main's checks found more. Each was
probed by reblessing the whole corpus under it, then checking the forms
corpus and the mutation's own tests.

## Decision

- `by-ref-back-lost` and `impl-trait-hides-held-into-iter` change only
  how the JS reads. Under each, the JS is still right:
  - `$lent(Iterator.from(it))` is `it`, since `Iterator.from` gives an
    iterator back.
  - A JS iterator with dictionaries made in the function, in place of
    `J` given as it is.

  So each now checks its case's snapshot (`snapshots: true`).
- `map-length-unknown` fails two cases that ask a map's length,
  `map_stepped_from_its_back.rs` and `owned_maps_asked_their_length.rs`.
  Its tests now name them.
- `rev-of-own-back-reversed`: `rev_items`' arms of an `Iterator.from(x)`
  receiver are unreached. With either arm panicking, the corpus passes.
  The last arm is now what steps any other receiver from its back,
  `as_double_ended`'s, which refuses what it can't make. Nothing falls
  back to an array of the items read from the front and reversed, which
  would be a wrong answer.
- `impl-bound-held-by-itself`: whether an impl holds an `IntoIter` is
  now asked of each function or impl alone, not of all of an impl's
  methods. Nothing's JS depended on the impl-wide answer.
  `test/corpus/into_iter_held_by_a_sibling.rs` checks the shape it was
  for, an impl some of whose methods hold `A::IntoIter`, against native
  Rust.

- Five are caught by tests that reach them again:
  - `written-break-not-first`: React's `addValueToProperties`, whose
    `break 'desc` ends a case, in the port's performance-track test, as
    6ca2f9688 found too. The probes had missed it: the corpus doesn't
    reach it.
  - `dictionary-cache-runs-code`: the dictionary test's cycle of
    dictionaries alone now holds a generic impl's cache and a dictionary
    the other module reads. Under the mutation that cycle is refused.
  - `moved-into-owner-unheld`: `iterator_moved_into_a_holder.rs`, a
    `take` moved into `wrap(..)`'s `Holder` and stepped past its limit.
    Under the mutation the rest drops before the `println!`.
  - `null-nones-calls-unwritten`: lowering-nullable's nullable
    function's result, 0faaa845d's. A new nullable-reads test,
    `halved_lazy`, also reaches it: a call's `Option` kept in a variable
    that a nullable field alone reads, whose `?? null` is where it's made.
  - `unread-ref-held-at-write`: `if_let_mut_else_assign.rs`'s `named`.
    Under the mutation it's `const f = found; f.push(v)`, so the mutation
    checks snapshots.
- One goes with the code it guarded, as nothing's JS depends on it:
  - Tidy's "stored again" pass. The places that store a nullable value
    store it again themselves, so the corpus, the examples, the React
    tests and every test that mentions a nullable place are the same
    without it.

## Consequences

Those eleven mutations are caught or removed. The rest are listed on
#382 until they are too.
