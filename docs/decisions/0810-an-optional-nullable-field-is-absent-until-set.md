# 0810. An optional nullable field is absent until it's set

Status: Accepted. Extends [0275](0275-nullable-writes-null.md) and
[0772](0772-a-nullable-cell-field-writes-null.md), whose nullable field's
`None` was `null` where its struct is made too.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: an absent key,
`undefined` and `null` are each `None` to the program, which reads any of
them as one (ADRs 0030, 0280); only what JS sees of the object changes.

## Context

React's `createContext` makes its object without its DEV-only fields and
adds them in DEV:

```js
const context: ReactContext<T> = {
  $$typeof: REACT_CONTEXT_TYPE,
  ..
  Provider: null as any,
  Consumer: null as any,
};
..
if (__DEV__) {
  context._currentRenderer = null;
  context._currentRenderer2 = null;
}
```

Flow types them `_currentRenderer?: Object | null`: optional, and `null`
once set. The port's `#[rust_js::nullable]` `Cell<Option<..>>` fields had
one answer for a `None`, `null`, so the literal wrote them, `_currentRenderer:
null`, and DEV set them again: twice, and present in production, where
upstream's object hasn't them.

A plain `Option` field's `None` is left out where its struct is made
already (ADR 0280), but it's `undefined` where it's set, where React writes
`null`. The field is both: optional, `?`, and nullable, `| null`.

The user's decision: an optional field, absent.

## Decision

**`#[rust_js::optional]` on a `#[rust_js::nullable]` field says it's
absent until it's set, Flow's and TypeScript's `field?: T | null`.** Where
its struct is made, its `None` is left out, as any `Option` field's is
(ADR 0280); where it's set, its cell's `set`, `replace` or `take`, or
assigned, `context.label = None`, its `None` is `null`, as a nullable
field's is (ADR 0772). The two attributes are Flow's two marks, each
meaning what it does alone: `nullable` the `| null`, `optional` the `?`.

**A nullable field assigned is written as its cell's `set` writes it**,
every nullable field's, optional or not: `None` is `null`, an `Option`
that may be `None` `label ?? null`, and a `Some(..)`, or what's `null` or
a value already, as it is. It was `context.label = undefined`: ADR 0772
wrote `null` where a cell is set, not where a field is assigned through a
`&mut`, so "`null` where it's set" held of one way to set it and not the
other.

```rust
pub struct ReactContext<T: 'static> {
    ..
    #[cfg_attr(rust_js, rust_js::name = "_currentRenderer")]
    #[cfg_attr(rust_js, rust_js::nullable, rust_js::optional)]
    pub current_renderer: Cell<Option<&'static Unknown>>,
}

let context = ReactContext { .., current_renderer: Cell::new(None), .. };
if __DEV__ {
    context.current_renderer.set(None);
}
```

```js
const context = { $$typeof: REACT_CONTEXT_TYPE, .., Provider: null, Consumer: null };
if (__DEV__) {
  context._currentRenderer = null;
}
```

- Made with an `Option` that may be `None`, it's given it as it is,
  `{ label }`, `undefined` for `None`, as a plain `Option` field is; a
  `Some(x)` is `x`.
- Its value is `null` or a value only once it's set, so it isn't one a
  nullable field or parameter takes as it is: given one, it's `?? null`,
  `{ name: context.label ?? null }` (`bindings::null_where_made`, the one
  answer of whether a field's `None` is `null` from where it's made on,
  which making, `nulled` and ADR 0781's `null_nones` ask).
- Read, absent, `null` or set, it's the `Option` it holds: `is_none()` and
  `match` test `== null`, and `==` of two treats `undefined` and `null`
  alike, as every `Option` read does (ADR 0030).
- Its `.d.ts` is a nullable field's, `field?: T | null | undefined`: one a
  caller may leave out takes `undefined` too (ADR 0196).
- **`#[rust_js::optional]` on a field that isn't nullable is a compile
  error**, as an attribute that does nothing is (ADR 0586): a plain
  `Option` field's `None` is left out already.

  ```
  rust-js: `#[rust_js::optional]` is of a `#[rust_js::nullable]` field, whose `None` is then left out where its struct is made and `null` where it's set; an `Option` field's `None` is left out already
  ```

## Why

- **Upstream's object**: no DEV field in production, `null` in DEV, each
  written once.
- **The marks people know**: Flow's and TypeScript's `?` and `| null`,
  one attribute each; a field with neither is a plain `Option`'s, with
  `nullable` alone always there, `null` or a value.
- **One owner**: whether a field's `None` is `null` where it's made is
  `null_where_made`'s; its writes, a cell's and an assignment's alike,
  `is_nullable`'s, each through `js::Storage::NullForNone.written` (ADR 0791).

## Alternatives

- **`#[rust_js::nullable(optional)]`**: a function's `nullable(..)` names
  its parameters; on a field the same form would name a mode. Rejected.
- **`optional` alone meaning nullable too**: its name says nothing of
  `null`, and a reader of Flow's `?: T | null` would look for both marks.
- **Every nullable field left out where it's made `None`**: react.dev's
  props to Next.js, `errorMessage: null` (ADR 0275), and React's own
  `Provider: null`, `owner: null` (ADR 0781), are there with `null`.

## Consequences

- A comment before an optional field made `None` in the literal goes with
  the field, which isn't written, as one before any field left out does
  (ADR 0280).

## What holds it

- `test/lowering-nullable.test.ts`, "an optional nullable field is left
  out where it's made and null where it's set": made with `None` and set
  in DEV, made with an `Option`, a `None` variable, given to a nullable
  field, assigned `None`, an `Option` and a `Some`, its `.d.ts`, and its
  reads, `is_some`, `is_none`, `==` and `match`, absent, `null` and set,
  each run.
- `test/diagnostics.test.ts`, "an optional field that isn't nullable".
- Mutations `optional-field-made-null`, `optional-unnullable-unrefused`
  (`bindings.rs`), `optional-field-made-null-in-literal`
  (`aggregates.rs`), `optional-field-read-nulled` (`mir/rvalues.rs`),
  `null-none-given-optional` (`mir/patterns.rs`),
  `nullable-assigned-undefined`, `nullable-assigned-some-coalesced`
  (`mir/assign.rs`).
