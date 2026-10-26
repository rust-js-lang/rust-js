# 0935. `js::NULL` is `None`, and `null` where it's written

Status: Accepted, the user's choice of 2026-10-26. Extends
[0586](0586-a-nullable-field-is-an-option.md) (a nullable `const`) and
[0225](0225-unknown-values.md) (`js::Unknown`).

Case ([0262](0262-when-rust-and-js-disagree.md)): N where Rust reads it:
`null` and `undefined` are each `None` to the program, which tests `!=
null` and compares `(a ?? null) === (b ?? null)` (ADRs 0030, 0612); C
where JS reads it, which tells `null` from `undefined` and sees a key
that's there: the program says which it means.

## Context

React's Fiber has fields typed `any` upstream, `memoizedState`,
`memoizedProps`, `updateQueue`, `stateNode` among them, which its
constructor makes `null` and which then hold exactly what JS writes,
`undefined` too:

```js
this.memoizedState = null;
..
workInProgress.memoizedState = nextState; // whatever a hook gave
```

The port's `Cell<Option<&'static Unknown>>` had two forms, neither
right:

- **A plain `Option` field made `None`** is left out of the object
  literal (ADR 0030). React's dev mode can't add the key later, its
  objects' shapes kept, and 1220 of its tests failed.
- **A `#[rust_js::nullable]` field** writes `?? null` on every store
  (ADR 0772), so a stored `undefined` reads back `null`: a wrong answer,
  where React tells them apart.

## Decision

The user's decision: **the js crate has `js::NULL: Option<&'static
Unknown>`, `None` in Rust, written `null`**, beside `js::Unknown` and
`js::cast`:

```rust
pub struct Fiber {
    pub memoized_state: Cell<Option<&'static Unknown>>,
}

Fiber { memoized_state: Cell::new(js::NULL) }
fiber.memoized_state.set(next_state);
```

```js
({ memoizedState: null });
fiber.memoizedState = nextState;
```

- **It's the js crate's `const`, `#[rust_js::nullable]`**, the meaning a
  nullable `const` has already (ADR 0586, amended): its `None` is
  `null`. A binding crate is never compiled to JS, so a use of one of its
  constants is its value, written in place (ADR 0031); a nullable one's
  is stored as a nullable place stores it, `null` (`named_const`).
- **A constant written of it is written as it's written**, `const EMPTY:
  Fiber = Fiber { memoized_state: Cell::new(js::NULL), .. }`'s `{
  memoizedState: null }`, not rustc's value of it, whose `None` would be
  left out: it's of a constant that isn't rustc's value, as one of a
  library's flags is (ADR 0031, amended).
- **The field stays a plain `Option`**: what's stored there later is
  stored as it is, `undefined` too. One meaning each: `js::NULL` says
  "this `None` is `null`", where it's written; `#[rust_js::nullable]`
  says "this place's `None` is always `null`".
- **Reading it is reading any `Option<&Unknown>`**: `is_some()` is `!=
  null`, `==` is `(a ?? null) === (b ?? null)`, `None` of either, as Rust
  reads both. `js::strict_equals(x, js::NULL)` is `x === null`, JS's.

## Why

- **It's what upstream writes**, `null` where it's made, and keeps what
  JS stores, which no attribute of the field can say: a field's attribute
  is of every write to it.
- **No dialect**: in Rust it's `None`, an `Option` std's code reads as
  one; only the JS it's written as says `null`.

## Alternatives

- **`#[rust_js::nullable]` on the field, stores passed through**: a
  nullable field would mean `null` at its making and as-is after, a
  second meaning of the attribute, where its `None` is `null` everywhere
  else.
- **A field attribute, "made null"**: an attribute per field for what
  one value says where it's written, and none for a `None` given
  elsewhere, an argument or a local.
- **`Some(js::unknown_of(js::Kind::Null))`**: a `Some` to Rust, whose
  `is_some()` is true natively and false in JS: a wrong answer.

## What holds it

- `test/lowering-nullable-constants.test.ts`, "js::NULL is null where
  it's written, and what's stored after is as it is": a struct made, a
  constant, a thread-local, a store and a reset, run as JS, an
  `undefined` stored kept so and read as `None`.
- Mutations `js-null-written-undefined` (`lower.rs`) and
  `js-null-constant-by-value` (`lower/pipeline.rs`).
