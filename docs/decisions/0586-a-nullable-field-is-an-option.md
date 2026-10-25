# 0586. `#[rust_js::nullable]` is of a field that's an `Option`

Status: Accepted. Extends [0196](0196-typescript-declarations.md)'s
nullable fields and [0275](0275-nullable-writes-null.md). Amended by
[0772](0772-a-nullable-cell-field-writes-null.md): or a `Cell` of one. Amended: or
a `Vec` of them, each item's `None` `null`.

Case ([0262](0262-when-rust-and-js-disagree.md)): none; a program rustc
accepts that rust-js can't give its meaning is refused.

## Context

`#[rust_js::nullable]` says a field's `None` is `null`. React's
scheduler's port put it on a `RefCell<Option<Callback>>`: the attribute
was taken of the cell, `({ value: callback } ?? null).value`, and of no
`None`. A function's `nullable` naming what isn't an `Option` was already
an error; a field's wasn't.

## Decision

**`#[rust_js::nullable]` of a field that isn't an `Option` is a compile
error**, as a function's is:

```
rust-js: `#[rust_js::nullable]` is of a field that's an `Option`, whose `None` is `null`; this one isn't
```

## Why

- **An attribute that does nothing it says is a silent difference**,
  which the reader can't see in the JS.

## What holds it

- `test/diagnostics.test.ts`, "a nullable field that isn't an Option".
- Mutation: the check gone.

## Amendment: of a `Vec<Option<T>>`, each item's `None`

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A `None` is `null`
or `undefined` alike to the program, which reads either as `None` (ADR
0030); only what JS reads of the items changes, which is what the field
says.

React's root keeps lane maps, arrays of 31 made by `createLaneMap(null)`,
whose items its JS tests `=== null`: `transitionLanes`, `hiddenUpdates`.
The port's `RefCell<Vec<Option<&'static Set<Unknown>>>>` wrote a cleared
lane `transitionLanesMap[index] = undefined`, which React's JS reads as
a set, and made the map `createLaneMap(undefined)`. `#[rust_js::nullable]`
was of a field that's an `Option`, so the program couldn't say its items
were.

**`#[rust_js::nullable]` of a field of a `Vec<Option<T>>`, or a `Cell` or
a `RefCell` of one, says its items' `None` is `null`**:

- **An item written by index, `v[i] = None`, through the field, lent,
  borrowed from its cell, held in a variable or not, is `null`**
  (`rvalues.rs`'s `field_item`, whose `index_mut` is of what
  `nullable_items_field` traces back to the field); and so is one
  `take`n or `replace`d, `v[i].take()` (`lends_field_item`).
- **A `None` given where its items are made is `null` where it's made**,
  as ADR 0781's `None` only nullable places read is: of the call that
  gives the field's value, through its cell's `new`,
  `RefCell::new(create_lane_map(None))` is `createLaneMap(null)`.
- **What's made otherwise holds what it's made of**: a closure's `None`,
  `(0..31).map(|_| None)`, a `vec![None, ..]`, an item `push`ed, is
  `undefined`, as any `None` is. Each is a step from the field to where
  the `None` is made that this doesn't take; `test/corpus/nullable_items`
  makes one so and reads it back as native Rust does.
- **The field itself is the array**, a `RefCell` of one the array (ADR
  0863), so a nullable `RefCell` of one needn't be plain.

```rust
pub struct FiberRoot {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub transition_lanes: RefCell<Vec<Option<&'static Set<Unknown>>>>,
}

root.transition_lanes.borrow_mut()[index] = None;
FiberRoot { transition_lanes: RefCell::new(create_lane_map(None)) }
```

```js
transitionLanes[$at(transitionLanes, index)] = null;
({ transitionLanes: createLaneMap(null) });
```

**Alternatives:**

- **Every `None` of a JS object's `Option`, anywhere, `null`:** one rule
  for every array and field, but it changes what every program's JS
  holds, the `undefined` JS's own optional values are, to say what a few
  of React's fields say.
- **A binding type for JS's nullable value, `js::Nullable<T>`:** a
  rust-js substitute for `Option`, which AGENTS.md rules out.
- **Refusing what makes the field's items otherwise:** a `None` a
  closure gives may still be one; refusing every value that isn't known
  to be made of `null`s would refuse a map a function of the crate makes
  with its own `None`s, which can't be seen from the field.

**What holds it:** `test/lowering-nullable.test.ts`, "a nullable field's
Vec of Options holds null for each None"; corpus `nullable_items` against
native Rust; mutations `nullable-item-written-undefined`
(`lower/mir/assign.ts`), `nullable-item-taken-undefined`
(`lower/mir/entries.ts`), `nullable-items-made-undefined`
(`lower/mir/patterns.ts`), `nullable-items-refused`
(`lower/bindings.ts`) and `nullable-items-cell-refused`
(`lower/drops/ref_cells.ts`).
