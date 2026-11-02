# 0996. A `take()` then tested is the place tested, taken where it's `Some`

Status: Accepted. The user's decision (2026-10-28, React port gap tk1).
Amends [0586](0586-a-nullable-field-is-an-option.md), amended, whose items'
`take()` stores `null`: a `#[rust_js::nullable]` field's own `take()` does
too.

Case ([0262](0262-when-rust-and-js-disagree.md)): None. Of a place
rust-js made, nothing can observe the order: a second read of a plain data
property or a variable gives the same value, and `None` written over
`None` changes nothing, each place having one representation of it.

## Context

React's prepareFreshStack, restartGestureRoot and GestureScheduler take a
callback out of a root's field and call it:

```js
const cancelPendingCommit = root.cancelPendingCommit;
if (cancelPendingCommit !== null) {
  root.cancelPendingCommit = null;
  cancelPendingCommit();
}
```

Their port writes `if let Some(cancel) = root.cancel_pending_commit.take()
{ cancel(); }`, which printed the take first, `const c = x.f; x.f =
undefined; if (c != null) { c(); }`: it writes the field even where it was
`None`. And a `#[rust_js::nullable]` field's `take()`, or `mem::take` of
one, wrote `undefined`, where its `None` is `null`, as each other write of
it makes it (ADR 0772): JS reading `root.f === null` saw `false`.

## Decision

**A `take()` of a place rust-js made, then tested by an `if let` or a
`match`, is the place tested, and taken in its `Some` branch**:

```rust
if let Some(cancel) = root.cancel.take() {
    cancel();
}
```
```js
if (root.cancel != null) {
  const cancel = root.cancel;
  root.cancel = undefined;
  cancel();
}
```

- **Lowering says which takes may move** (`Known::Taken`), where types
  are known, not from the JS's shape: `Option::take` and `Cell::take` of
  an `Option`, of a variable of the body's, a field of the crate's struct
  or a `Cell`'s, a path of variables and fields none of whose reads a
  getter answers (`js::taken_place`). Not a `RefCell`'s value read
  through its borrow, nor an item, `v[i]`, nor a binding's property.
- **Tidy moves it** (`tidy/options.rs::taken_options`, the "taken
  options" pass): `const c = p; p = None; if (c != null) { a } else { b }`
  is `if (p != null) { const c = p; p = None; a } else { b }`, of `c == null`
  too, only where `c` is read nowhere but `a`.
- **A nullable field's `take()` stores `null`**, `root.pending.take()`
  and `mem::take(&mut root.pending)`, through each `&mut *r` MIR makes of
  another (`lends_nullable_place`).

## Why

- **It's upstream's JS**, which the port compares line by line, and the
  user's decision: a write only where there was something to take.
- **One owner each**: lowering decides a take may move, by the place's
  type; tidy only moves what it's told.

## Alternatives

- **Keep the take first**: correct, and not what upstream writes.
- **Move any take**: a binding's property may be a getter or a setter,
  which a second read or a skipped write would show.

## Consequences

The corpus's `mut_ref_escapes.js` and the scheduler port's
`workLoop` read `if (holder.held != null) { const x = holder.held; ..`.
An `if let` whose branches give a value, `if let Some(p) = r.p.take() {
p.len() } else { 0 }`, keeps its statements, `if .. else { return 0; }`.

## What holds it

`test/lowering-takes.test.ts`, "an Option taken then tested is the place
tested, taken where it's Some", run with each place set and not; the
mutations `option-take-unmarked` (`lower/mir/entries.ts`),
`cell-take-unmarked` (`lower/std_types/cell.ts`), `taken-binding-read-after`
(`lower/mir/tidy/options.ts`), `taken-options-off`
(`lower/mir/tidy/stages.ts`), `taken-place-any` (`js/stmt_walk.ts`),
`nullable-field-taken-undefined` and `nullable-reborrow-unfollowed`
(`lower/mir/rvalues.ts`).

## Amendment: a take written over before it's seen is no write (2026-11-02)

rd524, P3. React's Flight client takes a chunk's listeners and makes it
fulfilled, its state a `RefCell` its class's fields flatten (ADR 1044):

```rust
let listeners = match &mut *chunk.state.borrow_mut() {
    ChunkState::Pending { value } | ChunkState::Blocked { value } => value.take(),
    _ => None,
};
*chunk.state.borrow_mut() = ChunkState::Initialized { value: text };
```

```js
if (chunk.status === "pending" || chunk.status === "blocked") {
  const old = chunk.value;
  chunk.value = null;
  listeners = old;
} else {
  listeners = undefined;
}
chunk.status = "fulfilled";
chunk.value = text;
```

Upstream's `const listeners = chunk.value; chunk.status = 'fulfilled';
chunk.value = text;` writes no `null`: nothing reads it.

**A take's write of `None` over a place rust-js made (`Known::Taken`),
a field of a variable, that the program writes again before anything
could see it is no write** (`tidy/options.rs::overwritten_takes`, the
"overwritten takes" pass, after "handles in place", which makes a
flattened state's whole write its fields'). Between it and the write
over it, past the end of the `if` branch it's in too, each statement
writes a variable other than the object's, or another field of the
object, of a constant or another variable: nothing that reads the
object, the old value or anything else, runs code or throws.

```js
if (chunk.status === "pending" || chunk.status === "blocked") {
  const old = chunk.value;
  listeners = old;
} else {
  listeners = undefined;
}
chunk.status = "fulfilled";
chunk.value = text;
```

- **The fact is lowering's** (`Known::Taken`, above): the place is a
  plain data property or a variable, never a binding's, which a setter
  may answer, nor a cell's value read through its borrow: run 8 thought
  this needed a fact of MIR's, but the one this decision gives the take's
  write already says the place is rust-js's own.
- **What it doesn't drop**: a write the program makes twice, `s.n = 1;
  s.n = 2;`, which is no take's; a take past which a call runs, the
  object or the place is read, or the variable is made another's
  (`slot = other`); one only another field is written after. The last
  but one has no mutation: lowering marks no take of a variable the body
  assigns again, so the pass's own test of it is never what refuses, as
  the corpus's `moved_on` shows.

Case ([0262](0262-when-rust-and-js-disagree.md)): none. Only a read of the
place could tell the `None` was there, and none runs before it's written
again.

Held by corpus `taken_then_overwritten` (the Flight client's take and
whole write; a struct's field taken and set again past another field;
one read between, in its branch too, given to a call, made another's,
only another field written after, and a field the program writes twice,
against native Rust), the changed `flattened_state_cell`,
`or_pattern_one_place` and `mut_param_moved_out`, and the mutations
`overwritten-any-write`, `overwritten-past-a-call`,
`overwritten-past-its-read`, `overwritten-same-field-between`,
`overwritten-branch-tail-unchecked`
and `overwritten-takes-off` (`lower/mir/tidy/options.ts`).
