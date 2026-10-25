# 0886. A function's evidence is one ordered list

Status: Accepted. The review's P4, first step: one authority for what a
generic function takes after its own parameters, before its JS shape
changes.

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable; the JS
is the same to the byte.

## Context

A generic function takes, after its own parameters, its caller's location
(ADR 0427), its const parameters' values (ADR 0107), its bounds'
dictionaries (ADR 0049), the facts it asks of its type parameters (ADR
0145) and its drops (ADR 0098), in that order. The order was written out
five times: by the body that names them (`evidence_params`), by a call
(`evidence_args_with`), by an impl's method's dictionary entry
(`method_evidence`), by a call through a dictionary of a method's own
(`own_evidence`) and by a copied default that takes them
(`default_method`). The last two named a method's own dictionaries in two
orders, `bounds()` filtered and `own_bounds()`, which agreed only because
those functions happen to.

## Decision

- **`evidence_slots(id, scope)` is the one list** (`traits/evidence.rs`):
  `Slot::Location`, `Const(index)`, `Bound(trait_ref)`, `Fact(slot,
  fact)`, `Drop(slot)`, of the whole function or of a trait method's own
  (`Scope::Own`), which a dictionary's caller gives. Each consumer gives
  each slot by its own rule (a name, a dictionary, a fact's value, a drop)
  and none orders them.
- `drops_taken` asks what's compiled here first, as `facts_asked` does: a
  std body compiled with the crate takes the drops this compile finds, not
  a manifest's.

## Consequences

- A new kind of evidence, or a type witness that groups a type parameter's
  (the review's P4), is one place to change.
- The mutations that held each loop hold each slot's value now.
