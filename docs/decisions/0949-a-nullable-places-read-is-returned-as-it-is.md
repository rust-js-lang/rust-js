# 0949. A nullable place's read is returned as it is

Status: Accepted. Extends [0578](0578-a-nullable-result-where-its-written.md),
a nullable function's result `null` for `None` where it's written, and
[0841](0841-a-nullable-place-stores-what-is-never-undefined-as-it-is.md),
a nullable place storing what's never `undefined` as it is, to a
function's result.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. What's returned is
`null` or the value, as `x ?? null` of it was.

## Context

React's `getSuspenseInstanceFromFiber` and `popSearchTarget` return a
nullable field's and a module variable's value:

```js
if (suspenseState !== null) {
  return suspenseState.dehydrated;
}
..
return searchTarget;
```

Their ports, `#[rust_js::nullable]` functions, returned
`suspenseState.dehydrated ?? null` and `searchTarget ?? null`: a nullable
function's result was `?? null` of anything not made where it's written,
though a `#[rust_js::nullable]` field's or thread-local's value is `null`
or a value already, as every write of it stores.

## Decision

- **A nullable function's result written of what MIR says is `null` or a
  value already** (`nulled`, the answer a nullable field's store asks:
  a nullable field's read, a `Some(..)`, a nullable function's result),
  **is as it is**.
- **A nullable thread-local's `get` is never `undefined`**
  (`Known::Defined`), as `local_facts` marks its call a nullable cell's
  (`QuietCells::nullable`), which its `set` already was.

## What holds it

- `test/lowering-nullable-reads.test.ts`.
- Mutations: `nullable-return-of-nulled-coalesced`,
  `nullable-local-get-unmarked`, `nullable-cell-get-undefined`.

## Amendment: one owner of what a nullable thread-local's read is (2026-11-03, #539)

`cell.rs` marked a nullable thread-local's `get()` `Known::Defined` as it
lowered it, and `mir/nulled.rs` said the same of its call where MIR writes
it (`stored_given`, rd523, rd632): two owners of one rule. And
`Known::Defined` was read as there by `Expr::presence` too, a wrong answer
(ADR 0910, amended).

**`nulled::reads_nullable_local` is the one answer**: whether the call a
block ends with, written to an `Option`, is a nullable thread-local's whole
read, `get()`, `take()`, `replace(..)` or `with_borrow(Clone::clone)`, as
`local_facts` finds them (`QuietCells::nullable`). `stored_given` asks it,
and where MIR lowers the call, its value is marked `Known::Defined`
(`mir/structure.rs`), never `Known::Present`: never `undefined`, but maybe
`None`. `cell.rs` marks none of them now; it still writes each of its
`set`s `null` for `None` (ADR 0586, amended). The JS is the same:
`return CURRENT;`, `return searchTarget;`, `fiber != null ? CURRENT :
null`.

Mutations: `nullable-local-read-unknown` and `nullable-local-read-unnulled`
(`mir/nulled.ts`), `nullable-local-read-unmarked` and
`nullable-local-read-present` (`mir/structure.ts`) in place of
`nullable-cell-get-undefined`, whose code is gone.

Not yet: a write through a borrow of a nullable place, `CURRENT.with(|c|
c.set(None))` or `TARGET.with_borrow_mut(|t| *t = None)`, stores
`undefined`, so such a read can be `undefined` where this says it isn't
(#541).

## Amendment: what's read is what each write stores (2026-11-04, #541)

The "not yet" above is done. A write through a borrow of a nullable place
stores `null` for `None`, as a direct one does ([ADR 1078](1078-a-nullable-place-stores-null-however-its-written.md)), so a nullable
thread-local's whole read is `null` or a value, as this ADR takes it. That
covers what `with` and `with_borrow_mut` lend, and a cell kept as one,
`$nullableCell(null)`. The reads' answer, `nulled::reads_nullable_local`, is
as it was.
