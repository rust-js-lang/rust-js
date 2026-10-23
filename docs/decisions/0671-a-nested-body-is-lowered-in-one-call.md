# 0671. A nested body is lowered in one call

Status: Accepted.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: how the compiler is
built; no program's JS changed.

## Context

A body lowered inside another, a generic impl's constant or a trait's
default copied into an impl, replaced part of `FnCx` as it was entered
(`enter_body`) and gave it back as it was left (`leave_body`). Each caller
did both, and a trait's default saved and restored more around them on
its own: the item's const parameters, its caller's location and how many
facts it was given. Its order was its own, save, enter, lower, restore,
`?`, leave, so a `?` between could return without leaving: right only
because the whole item fails with it, which a reader had to know. A field
added to what a body is given meant auditing each caller's order.

## Decision

**`lowered_in(body, owner, nested, |cx| ..)` is the one way into a
nested body and out: it enters it, runs the work, and leaves it, an error
too, giving back what the work gave and the drop functions the body
declares. `enter_body` and `leave_body` are `bodies.rs`'s alone, which
Rust's visibility holds.**

- **What a copied default is given is its frame's**: its evidence,
  arguments, const parameters, caller's location and the facts of its own
  type parameters are `DefaultGiven`, `Nested::Default`'s, put in place by
  `enter_default` and given back by `leave_default`, facts truncated to
  what the item had.
- **What survives is said apart**: the work's result and the hoisted
  declarations are returned; the names a body takes stay taken.
- **A body that fails to start** still leaves the state as it is, as its
  whole item fails: that contract is `enter_body`'s, in one place.

## Consequences

- A trait default's facts are worked out before it's entered, not pushed
  onto the item's and taken off after.
- Held by `scripts/mutations/lower/traits/given.ts`: facts and const
  parameters that stayed given past a default break the corpus.
