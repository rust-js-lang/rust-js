# 0712. What lowering a function finds of drops is returned with it

Status: Accepted. Amends [0190](0190-generic-no-destructor.md) and
[0300](0300-drops-by-need.md), whose findings each function wrote into
the crate's facts.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; the JS and the
diagnostics are the same, byte for byte, across the corpus.

## Context

A review of lowering's contracts (2026-10-24) found `CrateFacts`, "the
analysis's inputs, read only", holding three cells each function wrote as
it was lowered:

- `no_drops`, the type parameters each function takes no destructor of;
- `drop_checks`, the calls of the crate's functions to check once all are
  lowered (ADR 0190);
- `drop_uses`, what the functions that take only the drops they use did
  with them (ADR 0300).

So to know what lowering one function did, a reader had to know the
crate-wide computation it fed. The pipeline already had the pattern for
this: `LoweredFn` returns its `Dependencies`, which the pipeline gathers.

## Decision

**What lowering a function finds of drops is its own, `DropsObserved`,
the drops module's, which the pipeline gathers of each function in the
order they're lowered, and resolves once all are.**

- **The crate's facts keep what's known before any function is lowered**:
  `TakesUsed`, the functions that take only the drops they use.
- **A function's context has `drops_observed`**, written by the drops
  module alone (`test/architecture.test.ts`'s owners): the no-drops it
  requires, the calls it makes to check, and of a function that takes only
  the drops it uses, those it used, those it passed on and their names.
- **The pipeline gathers each function's after it's lowered, whether it
  was or failed**, as the cells held a failed one's, so its calls are
  checked as each other's. Then `check_no_drops`, its fixed point
  unchanged, and `DropsObserved::keep`.
- No function's lowering reads another's findings, so where they're kept
  changes no answer.

## Consequences

- `CrateFacts` holds no cell of drops; one function's lowering is read
  with what it returns.
- Held by the corpus, the crate and library tests, and the mutations
  `drops-observed-dropped` and `drops-by-need-uses-unnoted`.
- A failed function's findings are gathered as before, but no program
  found reaches it: each refusal the "takes none" check made of the
  crate's own functions has since been lifted (ADRs 0098, 0561, 0690), so
  no test tells gathering a failed one's from not.
