# 0889. An expression's facts are a set

Status: Accepted. The review's P2, its foundation: "a facts type on every
node, generalizing `Known`". Amends [0871](0871-rev-is-stds.md)'s
`Known::InPlace`.

Case ([0262](0262-when-rust-and-js-disagree.md)): none observable; the JS
is the same to the byte.

## Context

What lowering knows of a JS expression that its kind doesn't say, a
getter's read, a value never `undefined`, one of the program's functions, a
`Rev` made in place, was one `Known` value per expression, `Nothing` among
them. So a fact hid another: `Expr::in_place()` marked a value only where
nothing else was known of it, and a value both `Defined` and made in place
lost one of the two, which is what #257's mark exists to prevent.

## Decision

- **`Expr::facts` is a `Facts`, a set of `Known`'s**, `Facts::NONE`,
  `Facts::of(k)`, `has(k)`, `with(k)` and `is_empty()`; `Known` has no
  `Nothing`. Every reader asks `has`, where it compared before.
- What sets one fact as the expression's alone, a binding's read
  `answered_by_a_getter()`, `Expr::read`, a presence test, still does, and
  `defined()` still marks only what nothing else is known of, so each
  expression has the facts it had: the JS doesn't change.
- **`in_place()` adds its mark beside what else is known**, the first
  fact that coexists with another.

## Consequences

- A new fact of Rust's, carried from MIR to tidy, is a variant of `Known`,
  not a choice of which fact wins: the step P2's oracles (`has_effects`,
  `may_throw`, `reads_same`) need to ask Rust's facts, not the JS's shape.
- The mutations that quoted `known == Known::X` quote `facts.has(..)`; all
  are caught but `presence-truthy-unread`, a survivor on main already
  (#272).
