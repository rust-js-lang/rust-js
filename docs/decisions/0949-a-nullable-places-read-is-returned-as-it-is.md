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
