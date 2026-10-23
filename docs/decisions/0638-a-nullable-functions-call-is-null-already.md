# 0638. A nullable function's call is `null` already

Status: Accepted. Amends [0578](0578-a-nullable-result-where-its-written.md),
whose nullable result was `?? null` of every call.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, nothing the program
can observe: a `#[rust_js::nullable]` function's result is `null` for
`None` where it's written, so `x ?? null` of its call is `x`.

## Context

A `#[rust_js::nullable]` function's result is `null` for `None`, as
TypeScript's `T | null` (ADR 0578): where it's written, `nullable_written`
makes an `Option` that may be `undefined` `null`, `heap[0] ?? null`. A
call is such an `Option`, so React's `getComponentNameFromType`, which
returns its own call of a lazy component's type, wrote

```js
return getComponentNameFromType(resolved) ?? null;
```

where React writes `getComponentNameFromType(init(payload))`: the call is
the same function's, whose `None` is `null` already.

## Decision

**A nullable result written by a call of a `#[rust_js::nullable]`
function, this one or another, a binding too, is the call, with no `??
null`.** `nullable_written`, the one place a nullable result is written,
is told the function whose call gave the value (`made_by`, of the call's
terminator), and asks `bindings::nullable_result` of it, the one answer to
whether a function's `None` is `null`, as it asks of the body's own.

```rust
#[rust_js::nullable]
pub fn last_of(a: &[u32], b: &[u32]) -> Option<u32> {
    if b.is_empty() {
        return last(a);
    }
    last(b)
}
```
```js
export function lastOf(a, b) {
  if (b.length === 0) {
    return last(a);
  }
  return last(b);
}
```

A call of any other function's `Option`, whose `None` is `undefined`,
keeps its `?? null`; so does a variable a call set, which isn't told.

## Why

- **`?? null` can't change it**: the call is `null` or the value, and
  `null ?? null` is `null`.
- **A binding's is the same**: `#[rust_js::nullable]` on a binding says
  its JS gives `null` for `None`, as its TypeScript says, and rust-js
  takes it at its word, as it does a nullable parameter's.

## Alternatives

- **Tidy's or preparation's rewrite of `f(x) ?? null`**: it would have to
  know which function `f` is, which lowering knew and dropped.

## Consequences

- React's `getComponentNameFromType`'s `return getComponentNameFromType(
  resolved);`, and the guide's `lastOf`. No corpus program or snapshot
  changes.

## What holds it

- `test/lowering-options.test.ts`, "a nullable result of a nullable
  function's call is the call": a call of itself, of another, of a
  binding, and of a function whose `None` is `undefined`, which keeps its
  `?? null`, each run.
- Mutations in `scripts/mutations/lower/mir/assign.ts` and
  `scripts/mutations/lower/mir/structure.ts`.
