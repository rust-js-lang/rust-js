# 0772. A nullable `Cell` field's `None` is `null`, made and written

Status: Accepted. Amends [0586](0586-a-nullable-field-is-an-option.md),
whose `#[rust_js::nullable]` was of an `Option` field only, and
[0275](0275-nullable-writes-null.md), whose `null` was written where the
struct is made.

Case ([0262](0262-when-rust-and-js-disagree.md)): A: `null` and
`undefined` are each `None` to the program, which reads either as one
(ADR 0030); only what JS sees of the field changes.

## Context

React's shared internals are fields a renderer sets, `H`, `A`, `T`,
each `null` until it does; `createContext`'s `_currentRenderer` is set
to `null` again in DEV. A field set after the struct is made is a
`Cell`, `Cell<Option<&'static Dispatcher>>`, and `#[rust_js::nullable]`
of one was refused: ADR 0586 had it of an `Option` alone. The port held
plain `Option`s instead, which nothing can set.

## Decision

**`#[rust_js::nullable]` is of a field that's an `Option` or a `Cell` of
one. Its `None` is `null` where the struct is made, `renderer:
Cell::new(None)`, and where the cell is written, `set`, `replace` and
`take`:** `context.renderer = null`. A `Some(r)` written is `r`, and an
`Option` that may be `undefined`, `r ?? null`, as a made field's is
(`bindings::nullable_written`, the one rule for both). A comment before
the field stays before it.

```rust
pub struct Context {
    pub value: Cell<f64>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub renderer: Cell<Option<&'static Unknown>>,
}

pub fn make(value: f64) -> Context {
    Context {
        value: Cell::new(value),
        // These are circular
        renderer: Cell::new(None),
    }
}

pub fn reset(context: &Context) {
    context.renderer.set(None);
}
```

```js
export function make(value) {
  return {
    value,
    // These are circular
    renderer: null,
  };
}

export function reset(context) {
  context.renderer = null;
}
```

Its `.d.ts` is the `Option`'s, `renderer?: T | null | undefined`, as a
nullable `Option` field's.

## Why

- **The field React has**: `null` until set, set again to `null`.
- **One attribute, one meaning**: what's nullable is the field, whatever
  holds its `Option`.

## Alternatives

- **A nullable `RefCell<Option<T>>`**: still refused; a `RefCell` whose
  borrows are counted is an object of its own, not the field's value.

## Consequences

- The cell's own methods, called on the field, write `null`. A `&Cell`
  to it given elsewhere, whose `set(None)` doesn't know it's a nullable
  field's, writes `undefined`, which the program reads as `None` all the
  same (case A).

## What holds it

- `test/lowering-nullable.test.ts`, "a nullable Cell field's None is null
  where it's made and where it's written": made, set, set a `Some`, set
  an `Option`, taken and replaced, its comment and `.d.ts`, run.
- Mutations `nullable-cell-refused` and `nullable-null-unplaced`
  (`bindings.rs`), `nullable-cell-made-undefined` (`aggregates.rs`), `nullable-cell-set-undefined`
  (`std_types/cell.rs`), `nullable-cell-some-coalesced`
  (`body_queries.rs`), `nullable-cell-declared-undefined`
  (`declarations.rs`).
