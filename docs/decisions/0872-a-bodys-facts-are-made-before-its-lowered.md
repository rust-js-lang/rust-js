# 0872. A body's facts are made before it's lowered, and never changed

Status: Accepted. Extends [0364](0364-mir-lowering.md), whose `State` it
splits.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): how the lowering keeps
what it knows of a body; no JS changes.

## Context

`mir::State` held about fifty fields, each a table some module filled:
what lowering a statement changes, `pending`, `in_place`, `copy_backs`, and
what was known of the body before any statement was lowered, its control
flow graph, the variables its patterns bind, what its writes say of its
`Option`s, where its offsets are units, the jumps the program wrote. Both
kinds were fields alike, so a reader of a module couldn't tell, without
reading `mir.rs`, whether a table it read could have changed since, and
`test/architecture.test.ts` listed, for each module of `mir/`, every field
it touched. The architecture review (P3) asked that the second kind be one
record, made before emission.

## Decision

**What a body's lowering knows of it before it writes any of it is its
`LocalFacts`, `mir/local_facts.rs`'s: made by `FnCx::local_facts` before the
first statement is lowered, its fields private there and read through
`State::facts`' accessors, so no module can change one.** Its awaits, its
promoted bodies, its graph, with the arms a guard that holds goes on to,
the rests a `let` binds, the locals a pointer is taken to, its range
patterns, the jumps the program wrote, its payloads, the temporaries its
`let` patterns take apart, the props JSX takes apart, its offsets, where
each `let` is, and what its writes say of its `Option`s.

- **What's found as its structure is made stays `State`'s**: where each
  labeled block a `break` leaves ends, `left`, was a field of the jumps;
  it's `State`'s own now, and the jumps are facts.
- **What a guard that holds binds is read in place of its variable**,
  `mir_held_guards`, given the bindings `local_facts` found, as before.
- The architecture test lists, for each module, the fields of `State` it
  touches but `facts`, which any module reads and none changes.

## Why

- A module that reads `state.facts.options()` knows, from its type, that
  nothing lowered before it changed it.
- One place makes them, in one order, before emission: `mir_function` is
  fourteen lines shorter, and the table of what each module touches is
  what emission changes alone.
- Proved by the corpus, the snapshots and the lowering tests, whose JS is
  the same to the byte.

## Consequences

- Of what `State` still holds, `boxes`, `borrowed_names`, `flags`, `drops`
  and `captures` are made before emission too, but of the names its
  parameters are given as they're lowered; they move to `LocalFacts` once
  naming is a fact of its own (done: the amendment below).

## Amendment: what naming settles is a fact too

The rest of P3. **`drops` and `captures` are `LocalFacts`'**, given it as
it's made: a body's drop elaboration is made before its locals are, and a
closure's captures are given it. **`boxes`, `borrowed_names` and `flags` are
`NamingFacts`'**, a record of its own in `local_facts.rs`, private fields
read through accessors, made once by `mir_function` where naming its
locals and its drop flags is done, `state.naming`: what the parameters are
called, which of them is a box their caller gives (ADR 0074), the names of
the locals borrowed, and each drop flag's name.

- `State` keeps what emission changes: `own_names`, which the
  temporaries it names join, stays there.
- The architecture test leaves out `naming` as it does `facts`, and holds
  that `state.naming` is assigned once, in `mir.rs`.
- Proved by the corpus, the snapshots and the lowering tests, whose JS is
  the same to the byte.
