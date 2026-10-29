# 1017. An `Option` of an `Option` held where it's made is its payload

Status: Accepted: the user's representation (2026-10-29), of each
value whose every write and read can be told, the box kept elsewhere. Amends
[0051](0051-generic-options.md) (a `Some` of what looks like `None` is
boxed). Follows [1015](1015-what-holds-an-identity-is-never-eq-of-its-contents.md),
whose Consequences left it open.

Case ([0262](0262-when-rust-and-js-disagree.md)): none: each of the three
values Rust tells apart is its own JS value, and each read is of what its
writes made, so every answer is Rust's.

## Context

React's `processDispatchQueueItemsInOrder` keeps the fiber of the
listener before, `Fiber | null`, or none yet:

```js
let previousInstance;
for (let i = dispatchListeners.length - 1; i >= 0; i--) {
  const {instance, currentTarget, listener} = dispatchListeners[i];
  if (instance !== previousInstance && event.isPropagationStopped()) {
    return;
  }
  ..
  previousInstance = instance;
}
```

Its port says the same in Rust, the field `#[rust_js::nullable]`:

```rust
let mut previous_instance = None;
for DispatchListener { instance, .. } in dispatch_listeners.iter().rev() {
    if previous_instance != Some(instance) && (event.is_propagation_stopped.get())() {
        return;
    }
    ..
    previous_instance = Some(instance);
}
```

`previous_instance` is an `Option<&Option<Rc<Fiber>>>`, whose `Some` may
hold a `None`. ADR 0051 boxes that `Some`, `$some(instance)`, so `None`
and `Some(None)` differ, and since ADR 1015 the compare read

```js
const right = $some(instance);
if (!(previousInstance == null || right == null
    ? previousInstance == right
    : $someValue(previousInstance) == $someValue(right)) && ..) {
```

A whole-type representation, every `Option<Option<T>>` with `Some(None)`
`null`, can't hold: generic code boxes, `$some` of its `T` as it runs,
and std's compiled bodies make and read `Option`s of their items, an
iterator's `next()` of a slice of `Option`s among them. A value crossing
from one to the other would be read wrongly, a box taken for `Some(Some(box))`
or a `null` for `None`.

## Decision

**A local `Option<Option<T>>` or `Option<&Option<T>>` whose `T` never
looks like `None` (`representation::nested_payload`), written only `None`,
`Some(..)` or another such, and read only by another such, by its
discriminant, by what its `Some` holds, or, through a shared reference, by
an `==` of two such, is its payload: its `None` `undefined`, its
`Some(None)` `null`, its `Some(Some(v))` `v`, never boxed.**

```js
let previousInstance;
..
  if (previousInstance !== instance && event.isPropagationStopped()) {
    return;
  }
  ..
  previousInstance = instance;
```

- **Where it's decided.** `mir/patterns.rs`'s `mir_nested`, a body's
  fact `OptionWrites::nested`, as its other `Option` facts are: each
  mention of such a local or its reference that isn't one of those is
  out, and so is each that's copied to or from, or compared with, one
  that's out. A parameter, the result, one with a destructor, one given
  to a function, returned, captured, matched against a constant, or
  borrowed `&mut` keeps ADR 0051's box.
- **What reads it.** `mir/assign.rs` writes `None` `undefined` and a
  `Some` what it holds, its `None` `null` (`Storage::NullForNone`), as is
  of a nullable field's or a reference to one (`stored_given`, which now
  follows a shared reference and a reborrow), `previousInstance =
  instance`, else `instance ?? null`; `mir/rvalues.rs` tests it `x !==
  undefined`; `mir/places.rs` reads what its `Some` holds as the value
  itself; `mir/dispatch.rs`'s `nested_equality` compares two by `===`
  where their payload's `==` is `===`, through `Box`es and uncounted
  `Rc`s, and else `a === undefined || b === undefined ? a === b : <a == b
  of the inner `Option`s>`.
- **One representation per value.** A local's values never meet code that
  boxes: what's out keeps the box all through, so no value is ever read
  in the other form.

## Why

- **It's upstream's JS**, `let previousInstance;`, `previousInstance =
  instance`, `instance !== previousInstance`, from the Rust a person
  writes for it.
- **It's exact.** `undefined`, `null` and the payload's values are three
  kinds no `T` that never looks like `None` makes, so `===` and `!==
  undefined` answer as Rust's `==` and `is_some()`.

## Alternatives

- **Every `Option<Option<T>>` so**, the user's representation as a type's:
  wrong where generic code meets it (above). Rejected in favor of the
  local, which is that representation wherever it can be told.
- **Convert at each generic boundary**, a box made of `null` going in, a
  `null` of a box coming out: every call, closure and std body returning
  or taking an `Option<P>` of a `P` that's an `Option`, `filter_map`'s
  too. Rejected: a second representation of one type the whole program
  would have to agree on.

## Consequences

- The port writes `Some(instance) != previous_instance` for upstream's
  operand order, `instance !== previousInstance`.
- `matches!(last, Some(None))` reads `last !== undefined && last == null`.
- `last == Some(None)`, a constant MIR promotes, keeps the box: the
  constant isn't a local made here.

## What holds it

- Corpus `nested_option_held_as_payload`: React's loop over listeners of a
  nullable field and of a plain one, `match`, `if let` and `matches!` of
  one, payloads compared by `===`, through a `Box`, and by `$eq`, and one
  given to a function, which keeps its box, against native Rust.
- `test/crates/pairs/library_eq`: a consumer's loop over a library's
  listeners, whose field is nullable, `previousInstance !== instance` and
  `previousInstance = instance`, against native Rust.
- `docs/guide/interop/types.md`'s example, held by `test/docs.test.ts`.
- Mutations `nested-options-boxed`, `nested-option-mentions-unchecked`,
  `nested-option-copies-unlinked` (`lower/mir/patterns.ts`),
  `nested-option-none-null`, `nested-option-some-none-undefined`
  (`lower/mir/assign.ts`), `nested-option-test-loose`,
  `shared-ref-given-unknown`, `reborrow-given-unknown`
  (`lower/mir/rvalues.ts`), `nested-option-eq-loose`,
  `nested-option-eq-nones-unseen`, `nested-option-box-unpeeled`
  (`lower/mir/dispatch.ts`) and `nested-option-payload-unboxed`
  (`lower/mir/places.ts`).
- React's port with DOMPluginEventSystem installed: the seven DOM event
  suites pass, 9 suites, 401 tests.
