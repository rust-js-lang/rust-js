# 0993. What compiled `rev()` is given is made knowing its back where it's made

Status: Accepted. Amends [0871](0871-rev-is-stds.md) and
[0980](0980-std-s-struct-given-to-rev-is-given-as-it-is.md). Found by
#382's triage of `adapter-given-to-rev-unknowing`.

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable. The
same program, written once; one rule fewer.

## Context

ADR 0871 made std's `rev()` compiled. A std adapter given as it is to it,
a JS iterator, was made knowing its back where it's given:
`as_double_ended` in `stepping.rs`, for a parameter `rev()` steps from its
back by its dictionary. ADR 0980 kept std's compiled structs out of it.

The back-stepped analysis already makes each such source knowing its back
where it's made, a held `rev()`'s included (ADR 0545). So the rule only
ran on values that already knew their back. On a `Vec`'s iterator held in
a MIR temporary, whose JS can be an array or its `$iter`, it wrapped it
again: `iterator_rev($iter($iter(louds(5))), ..)`.

## Decision

The rule is removed, with `steps_back_by_dictionary`, which only it read.
What compiled `rev()` is given is made knowing its back where it's made.

## Consequences

Four snapshots read `iterator_rev($iter(louds(5)), ..)`. The corpus, the
forms corpus and the snapshots run as before; `held_revs_of_lazy_sources`
checks held `rev()`s of a filter, a `filter_map` and maps. The mutations
`adapter-given-to-rev-unknowing` and `compiled-struct-made-knowing` go
with it (#382).
