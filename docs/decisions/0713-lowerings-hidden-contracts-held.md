# 0713. Lowering's hidden contracts, held where they're relied on

Status: Accepted.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; the JS is the
same, byte for byte, across the corpus.

## Context

A review of lowering's contracts (2026-10-24) found three rules each
caller had to remember, which nothing held:

- **A std call's kinds** (`mir_std_call`): seven handlers, `fmt`'s,
  text's, a map's, a `Vec`'s, an iterator's, a place's and a value's, are
  tried in turn, each given the call's arguments to take from, the body's
  state and what's written. One that says `None` must have taken no
  argument and written nothing, or the next is given what's left.
- **A function as a value** (`mir_called_value`): its arrow is lowered with
  what's made and not yet used, the cleanup that unwinds to, and the
  copies back a call owes all its own, and the enclosing ones put back
  after, by hand, before each `?`.
- **A body that fails to start** (`enter_body`): it changed the frame and
  the item's scope before its drops' facts, which may fail, and left them
  changed; safe only as the whole item then fails.

## Decision

- **A std call's kinds are called through one loop**, `StdHandler`'s, and
  one that declines is checked to have taken no argument, written no
  statement and made no temporary, an error naming it otherwise. The
  corpus and the lowering tests find none that does. The review's further
  step, the call routed to its one kind before any is given its
  arguments, is a rewrite of each kind as a decision and its lowering;
  it's left for its own change, with no second list of what each takes.
- **`in_fresh_evaluation(state, lower)`** lowers in an evaluation of its
  own and gives the enclosing one back however `lower` ends, with the
  copies back its own calls owe, which its caller writes in the arrow, as
  before. `mir_called_value` is its one user; the save and restore by
  hand are gone.
- **A body that fails to start leaves the enclosing one's context as it
  was**: `restore_enclosing`, which `leave_body` uses too, and
  `enter_body_drops` gives a default's drops back where its facts fail.

## Consequences

- Held by the corpus, `test/corpus/function_values_in_calls.rs`, and the
  mutation `fresh-evaluation-pending-lost`. The enclosing cleanup and
  copies back are put back too, but no program reaches them: a call's
  copies back are made once its operands are, so none is owed while a
  function value among them is made.
- A failed start is still the item's failure; restoring is for what may
  come to call it otherwise.

## Amendment: a declining kind writes apart

The check of a kind that declines compared lengths: it couldn't tell an
existing statement changed (a review, 2026-10-24). **Each kind writes
into a list of its own, which is the body's once it takes the call**, so
one that declines can't have changed what's written, and is checked to
have written nothing, taken no argument and made no temporary. The JS is
the same, byte for byte, across the corpus; the mutation
`std-kind-writes-lost` holds that what a kind writes is kept. Routing a
call to its one kind before any is given it is still the stronger step,
left as before.
