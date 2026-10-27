# 0861. A function's own nullable parameter is `null` where it's given or set `None`

Status: Accepted. Extends [0275](0275-nullable-writes-null.md)'s
`#[rust_js::nullable(name)]`, a binding's parameter, to a function of the
crate's own, and [0772](0772-a-nullable-cell-field-writes-null.md) and
[0841](0841-a-nullable-place-stores-what-is-never-undefined-as-it-is.md)'s
nullable places to it.

Case ([0262](0262-when-rust-and-js-disagree.md)): A, decided by the user
(2026-10-25): an `Option<&Unknown>`'s `None` given to a place typed `T |
null` is JS's `null`, elsewhere `undefined`. The program reads `null` and
`undefined` alike as `None` (ADR 0030), and an any-shape value's `None` is
either already (ADR 0676); only what JS is given changes.

## Context

React's `mapIntoArray` (ReactChildren.js) makes `undefined` and a boolean
`null`, and calls the callback with it:

```js
if (type === 'undefined' || type === 'boolean') {
  // All of the above are perceived as null.
  children = null;
}
..
let mappedChild = callback(child);
```

Its tests say so, `expect(callback).toHaveBeenCalledWith(null, 5)` of a
child that was `true`, `false`, `null` or `undefined`. The port's
`children = None;` was `children = undefined;`, as every `None` is. A
`#[rust_js::nullable(children)]` on the function was taken, and its call's
`map_into_array(None, ..)` was `null` already (ADR 0781's `null_nones`,
which asks any function's), but not where the body set it.

## Decision

**`#[rust_js::nullable(name)]` on a function of the crate names a
parameter of its own, `T | null` as a nullable field is: a `None` written
to it is `null`, where a call gives it and where its body sets it.** The
body sets it as it sets a nullable field, `Storage::NullForNone`'s: a
`None` `null`, a `Some(..)` or what's `null` or a value already as it is,
an `Option` that may be `undefined` `?? null`.

```rust
#[cfg_attr(rust_js, rust_js::nullable(children))]
fn map_into_array(mut children: Option<&'static Unknown>, ..) -> u32 {
    let r#type = js::type_of(children);
    if r#type == "undefined" || r#type == "boolean" {
        // All of the above are perceived as null.
        children = None;
    }
    ..
}
```

```js
function mapIntoArray(children, ..) {
  const type = typeof children;
  if (type === "undefined" || type === "boolean") {
    // All of the above are perceived as null.
    children = null;
  }
  ..
}
```

- **What a call gives it is as it is**, a variable, `mapIntoArray(child,
  ..)`, or a call's `Option`, `mapIntoArray(init(payload), ..)`; a `None`
  written there is `null`, `mapIntoArray(null, ..)`: the function's own
  body reads it, which reads `null` and `undefined` alike, where a
  binding's JS may not, whose argument is `child ?? null` (ADR 0275), and
  a call's `Option` only it reads `f() ?? null` (ADR 0841,
  `mir_null_nones`, which now tells what it's stored in apart).
- **Its `.d.ts` is `T | null | undefined`**, as a nullable field's is
  `?: T | null`: a JS caller may give either.
- **One attribute, one meaning**: the parameter `T | null`, of a binding
  and of the crate's function alike; on the function, as a binding's is,
  since a parameter's own attributes aren't kept in a crate's metadata.

## Why

- **React's JS**, and what its tests see: the callback given `null`.
- **The decision the user made**: a nullable place's `None` is `null`.

## Alternatives

- **Coalesce what each call gives**, `mapIntoArray(init(payload) ?? null,
  ..)`: six `?? null` in React's function where upstream has none, of
  what its body reads either way.
- **`null` of every `Option<&Unknown>`'s `None`**: what isn't typed `T |
  null` stays `undefined`, as the user decided.

## What holds it

- `test/lowering-nullable.test.ts`, "a function's own nullable parameter
  set to None is null": `undefined`, `true`, `null` and a number given, a
  `None` given, and a parameter not marked beside it, each callback's
  argument checked.
- Mutations `own-nullable-param-set-undefined`,
  `own-nullable-param-any-argument` (`scripts/mutations/lower/mir/assign.ts`)
  and `null-nones-own-param-call-coalesced` (`lower/mir/patterns.ts`).

## Amendment: a nullable parameter is `T | null` as declared, and what only nulls is returned as it is

The user's decision (2026-10-27), Case A ([0262](0262-when-rust-and-js-disagree.md)):
React's ports printed `?? null` where upstream passes a value on:

| Rust | Was | Upstream |
|---|---|---|
| `capture_commit_phase_error(current, nearest_mounted_ancestor)`, both `#[rust_js::nullable(..)]` | `captureCommitPhaseError(current, nearestMountedAncestor ?? null)` | `captureCommitPhaseError(current, nearestMountedAncestor)` |
| `SEARCH_TARGET.set(target)` of a nullable parameter | `searchTarget = target ?? null` | `searchTarget = target` |
| `SEARCH_TARGET.with_borrow(Clone::clone)` | `return searchTarget ?? null` | `return searchTarget` |
| `let mut root = None; .. root = Some(Rc::clone(&n)); .. root` | `let root; .. return root ?? null` | `let root = null; .. return root` |

A nullable parameter's `None` was `null` where the crate's calls give it,
but its `.d.ts` said `T | null | undefined`, so a JS caller might give
`undefined`, which `?? null` turned to `null` wherever it went next.

**A `#[rust_js::nullable(..)]` parameter of the crate's function is `T |
null` as declared**: its `.d.ts` is `fiber: Fiber | null`, never left
out, and its body takes it as `null` or a value already
(`stored_given`'s `Nulled`), given on to a nullable parameter or stored
in a nullable place as it is. A JS caller that gives `undefined` breaks
the declaration, as it would TypeScript's. The body still reads `null`
and `undefined` alike as `None` (ADR 0030).

And what a nullable function returns that holds only `null`s and values:

- **a nullable thread-local's whole value cloned**,
  `KEY.with_borrow(Clone::clone)`: every write of it stores `null` for
  `None` (ADR 0586, amended), and ES modules give importers no way to set
  a module's variable, so it's `null` or a value (`QuietCells::nullable`,
  `Known::Defined`);
- **a `let` of `None` the function returns**, set only to `Some(..)`s,
  one MIR makes in a temporary and moves in too: the return is a
  nullable place it's read by (`null_nones`, ADR 0781), so its `None` is
  `null` where it's made, `let root = null;`, and it's returned as it is
  (ADR 0957's let set only to values, extended to one that starts
  `None`).

A nullable field's read kept in a `let` and returned keeps its `?? null`:
JS may have set the field `undefined`.

Held by `test/lowering-nullable-reads.test.ts`'s "a nullable parameter,
a nullable thread-local's clone and a let of None returned are as they
are" and `test/lowering-module-variables.test.ts`'s "a nullable
thread-local's None is null where it's made and set", its `.d.ts`
`setCurrentFiber(fiber: Fiber | null)`, run in JS; and the mutations
`own-nullable-param-maybe-undefined` (`mir/rvalues.ts`),
`nullable-local-clone-unread` (`mir/local_facts.ts`),
`nullable-local-clone-maybe-undefined` (`std_types/cell.ts`),
`nullable-return-not-a-nullable-read`, `some-in-a-temporary-not-a-write`
(`mir/patterns.ts`) and `nullable-param-declared-optional`
(`declarations/functions.ts`).

