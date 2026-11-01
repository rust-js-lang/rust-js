# 1058. A held borrow meets only borrows of its own class

Status: Accepted (rd482, P2, 2026-11-02). Amends
[0863](0863-a-refcell-of-an-object-is-the-object.md) (a borrow is
counted where it's held while code that may borrow its cell runs) and
[0580](0580-a-call-asks-of-a-cell-only-what-it-may-borrow.md) (what a
call may borrow, by its cells' types). Amended (2026-11-02, rd535): a
`From` into an untagged enum runs nothing.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, no observable
difference. A borrow left uncounted is one no other borrow can meet, so
no Rust panic is lost; one that may be met is counted, and throws where
Rust panics.

## Context

Since #445 a cell's flow (`CellFlow`, Steensgaard's unification over the
crate's MIR) gives each borrow the class of cells it can be of. That told
only which borrows a counted one makes checked. Whether a borrow is
counted at all was still asked by type: held while a call runs that
borrows any cell of its type, mutably, it was counted, even where that
call can only borrow another cell. A queue's items walked while another
queue's are pushed to,

```rust
fn drain_into(from: &Queue, to: &Queue) {
    for &item in from.items.borrow().iter() {
        push(to, item * 10);
    }
}
```

printed `$borrow(from.items, true)` and a `try`/`finally`; and a
flattened state ([1044](1044-a-flattened-state-is-its-classs-own-keys.md)),
which counts no borrows, was refused held so.

Separately, a `&mut` binding of a flattened state's field, `Pending {
value } | Blocked { value }`, held its chunk under a name of its own:
`const value = chunk; value.value ??= [];` (ADR 1045's consequence).

## Decision

**A borrow's class is part of what it is** (`Borrow::class`): each body's
borrows of a cell of a type, the held one's too, carry the class their
flow says (`CellFlow::class_of`, by where the call is written, as
`ref_cells` already read it), and two borrows meet only where their
classes may be one cell (`may_meet`): two of the crate's own only of one
class, and one of the crate's own never one code rust-js can't see may
reach. A borrow whose class isn't known meets any of its type, as before.

**What code rust-js can't see may borrow of the crate's own cells** is
what the crate's bodies that code may call borrow, and what they run that
rust-js sees (`CellBorrows::unseen`): a thread-local's cell, which no
value given reaches, borrowed by a callback by its key, is still met.

So `drain_into` is a plain loop, `const items = from.items; for (..)
push(to, ..)`, and its queue isn't checked in `main` either.

**A `&mut` field binding's object, fixed where it's borrowed**
(`fixed_kept`), is a name the lowering made: read as the variable it
holds where nothing sets that variable meanwhile (`alias_variables`), so
a flattened state's `value` is `chunk.value`, as upstream's `case
PENDING: case BLOCKED:` reads it.

## Why

- It's the question the counts answer: whether two borrows of one cell
  can overlap. The flow already says which cell each can be; asking it by
  type left checks, and refusals, where no two borrows meet.
- The class rides on the borrow, so every question asked of two borrows,
  a call's, a body's summary's, a `with_borrow`'s, asks it alike.

## Alternatives

- **A summary of classes each body borrows, kept apart from its types'**:
  two summaries of one thing, each with its own fixpoint.
- **Context-sensitive classes, a constructor's each call's own**: a
  class's instances made by `fn new() -> Self` are one class, so React's
  chunks, all made by `createPendingChunk` and kept in one `Map`, are one;
  splitting them would need the allocation's caller in the class. Not
  done: a chunk's state held while a listener runs, which may be any
  chunk's, is a real overlap, which Rust's `RefCell` panics at too, and
  the Flight client writes [1055](1055-a-match-of-what-a-match-made-is-one-dispatch.md)'s
  form there.

## Consequences

- A flattened state read while another chunk is written is refused as
  before where both are made by its class's constructor, which makes them
  one class; a struct's `RefCell` fields made apart by literals are
  classes of their own.
- Held by the corpus's `held_borrow_of_own_cell`, against native Rust:
  `drain_into` and `copy_into` unchecked, a callback's `try_borrow_mut`
  of the walked cell, given as a capture or by a thread-local's key,
  finding it borrowed as Rust's does; `flattened_state_cell` and
  `or_pattern_one_place`, a field binding's `chunk.value`;
  `test/classes-flattened-state.test.ts`, a chunk's state held while the
  crate's code borrows the same chunk, refused. Mutations
  `classes-always-meet`, `own-class-unseen-asks-nothing`,
  `borrows-unclassed`, `held-borrow-unclassed` (`analysis/cell_borrows.rs`),
  `held-borrow-direct-unclassed` (`drops/facts.rs`), `fixed-alias-kept`
  (`tidy/bindings.rs`), `kept-object-named` (`mir/assign.rs`).

## Amendment: a `From` into an untagged enum runs nothing (2026-11-02, rd535)

React's Flight reply client makes a Blob of the chunks a BYOB reader read,
each a `BlobPart`, while its buffer is borrowed:

```rust
let parts: Vec<BlobPart<'_>> = buffer.borrow().iter().map(|&chunk| BlobPart::from(chunk)).collect();
```

`BlobPart::from` is webapi's, whose body rust-js doesn't compile, so the
call was code rust-js can't see, which may borrow any cell such code
reaches: the borrow was counted, `$borrow(buffer, true)` and a
`try`/`finally`, and so was every borrow of a cell of its type that such
code reaches, the Flight client's own `streamState._buffer`, `$borrow(buffer).slice()`
and `$borrowMut(buffer).push(..)`, in a module the reply client never
touches.

**A `From` into an untagged enum runs nothing** (`call_runs_nothing`,
asking `untagged::is_untagged_from`): ADR 0214 makes it the value it's
given where it's called, and its `from` is checked to be the variant of
its argument, so no code runs, a library's as the crate's. The borrow is
plain, `const parts = buffer.slice();`, and nothing else of its type is
checked for it.

Case N ([0262](0262-when-rust-and-js-disagree.md)): no borrow can meet
one held across a call that runs nothing, so no Rust panic is lost.

Held by `test/webapi-objects.test.ts` ("a borrow held across a From into
an untagged enum is plain"), run, and the mutation
`untagged-from-runs-code`.
