# 0586. `#[rust_js::nullable]` is of a field that's an `Option`

Status: Accepted. Extends [0196](0196-typescript-declarations.md)'s
nullable fields and [0275](0275-nullable-writes-null.md). Amended by
[0772](0772-a-nullable-cell-field-writes-null.md): or a `Cell` of one. Amended: or
a `Vec` of them, each item's `None` `null`; and of a thread-local, and a
binding's type parameter, below.

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

## Amendment: of a thread-local, and of a binding's type parameter

Case ([0262](0262-when-rust-and-js-disagree.md)): A where JS reads it
`== null`, and C where it's `=== null`: React's ReactCurrentFiber has
`export let current: Fiber | null = null`, set `null` again where it's
reset, and ReactFiberHiddenContext `createCursor(null)`. The port's
`thread_local! { static CURRENT: RefCell<Option<Rc<Fiber>>> = .. }` was
`export let current;`, set `current = undefined`, and a generic binding
`create_cursor<T>(default_value: T)` given `None` `createCursor(undefined)`;
`#[rust_js::nullable(default_value)]` of it was refused, its parameter no
`Option`.

- **`#[rust_js::nullable]` on a `thread_local!`'s static**, of a `Cell` or
  a `RefCell` of an `Option`, makes its `None` `null`, as a nullable
  field's: where it's made, `export let current = null;`, where it's set,
  `current = null;` (`set`, and a `Cell`'s `replace` and `take`), and to
  TypeScript, `export let current: Fiber | null;`. The `thread_local!`
  macro puts the attribute on its key, which says it (`nullable_local`);
  MIR's `QuietCells` says which calls write a nullable one's cell. Of a
  thread-local of anything else it's refused. A `None` a `with_borrow_mut`
  closure writes through its `&mut` is `undefined`, as any a closure
  makes is (this ADR's first amendment).
- **`#[rust_js::nullable(p)]` may name a binding's type parameter's
  value**, `default_value: T`: where it's given an `Option`, its `None` is
  `null`, `createCursor(null)`; any other value is as it is,
  `createCursor(0)`.

### What holds it

- `test/lowering-module-variables.test.ts`, "a nullable thread-local's
  None is null where it's made and set", "a binding's nullable type
  parameter is given null for None" and "a nullable thread-local of no
  Option is refused".
- Mutations `nullable-local-of-no-option-kept`,
  `nullable-type-parameter-refused` (`bindings.rs`),
  `nullable-type-parameter-of-any` (`mir/calls.rs`),
  `nullable-local-made-undefined` (`pipeline.rs`),
  `nullable-local-set-undefined`, `nullable-made-as-given`
  (`std_types/cell.rs`), `nullable-local-calls-unseen`
  (`mir/local_facts.rs`) and `nullable-local-declared-undefined`
  (`declarations.rs`).
