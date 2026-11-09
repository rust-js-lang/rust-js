# 1108. Code no run reaches, deleted or added, changes nothing

Status: Accepted (#1086; the user's queue of new nets, 2026-11-09).

Case ([0262](0262-when-rust-and-js-disagree.md)): none decided here. This
is a net: each wrong answer it finds is its own issue, decided there.

## Context

Equivalence modulo inputs (Le, Afshari and Su, PLDI 2014) found hundreds
of GCC's and LLVM's bugs: on a given input, code the run never reaches can
be deleted, or code added that never runs, and the program must print the
same. rust-js's cleanup passes and analyses react to code that never runs:
a branch's writes, a dead `&mut`, a closure never called, a drop on a path
not taken. The metamorphic net (ADR 1104) rewrites code that runs; EMI is
its complement.

## Decision

**Two rewrites more of the metamorphic net, `emi-prune` and `emi-insert`,
each against native Rust in both builds, by its harness and its known
list.**

- **What's reached** is found by a probe (`metamorph emi-probe`): each
  block numbered as it's written, a function's, a closure's, a branch's, a
  loop's, a match arm's, each saying `METAMORPH-EMI-HIT <n>` on stderr the
  first time it runs; none in a `const fn`, a `const`, a `static` or a
  binding to JS. The probe runs natively and as JS (`reached`,
  `test/metamorphic.ts`).
- **`emi-prune`:** each block neither run reaches made `unreachable!()`.
  A block only the JS reaches is kept: a difference decided and written
  down makes it, a panic's payload that downcasts as a `&str` and a
  `String` both (ADR 0692), where Rust's is one; native coverage alone
  flagged seven cases for it.
- **`emi-insert`:** code that never runs added, `if
  std::hint::black_box(false) { .. }`: after each `let` of a name a borrow
  of it, `&mut` where it's `mut`; and at `main`'s start a call of each of
  the case's free functions that take nothing.
- Like every rewrite, each is tried of every place, then of its first
  alone, and a variant rustc rejects is no variant. A push runs the net's
  sample, the nightly every case; `METAMORPHIC_REWRITES=emi-prune,emi-insert`
  runs these two alone.

## Consequences

Every case of the corpus, both rewrites, both builds, on the 12-core M6 in
12 parts: the first run, 200 s cold, pruning by native coverage alone, had
eight cases fail; with what the JS reaches too, none, 1,942 tests passing
in 65 s with the first run's runs kept. No wrong answer from what it deletes
or adds. Of the eight, seven were the payload difference, and one a wrong
answer:
`fmt::Arguments::as_str()` of a format with a runtime argument is
`Some(..)` in JS where native Rust's is `None`, filed for m1-lisa.

Held by `test/metamorphic.test.ts`' `emi-prune makes each block no run
reaches unreachable!()` and `emi-insert adds code never run`.
