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
