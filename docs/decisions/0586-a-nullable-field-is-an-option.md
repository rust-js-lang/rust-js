# 0586. `#[rust_js::nullable]` is of a field that's an `Option`

Status: Accepted. Extends [0196](0196-typescript-declarations.md)'s
nullable fields and [0275](0275-nullable-writes-null.md). Amended by
[0772](0772-a-nullable-cell-field-writes-null.md): or a `Cell` of one. Amended: or
a `Vec` of them, each item's `None` `null`; and of a thread-local, and a
binding's type parameter; and of a generic struct's field of a type
parameter's value, and a thread-local a generic constructor makes, below;
and of a thread-local of an `Option` itself, and what one gives, and of a
`const`, below; and a constant's struct's field, below; and each declared
by the aliases its type is written of, below. Extended by
[0935](0935-js-null-is-null-where-its-written.md): the js crate's
`js::null()`, a `None` of any `Option`, is `null` where it's written.

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

## Amendment: of a generic struct's field of a type parameter's value, and of a thread-local a generic constructor makes

Case ([0262](0262-when-rust-and-js-disagree.md)): A where JS reads it
`== null`, and C where it's `=== null`, as React's JS reads
`currentTreeHiddenStackCursor.current !== null`. A `None` is `null` or
`undefined` alike to the program (ADR 0030); only what JS reads changes,
which is what the attribute says.

React's ReactFiberStack, ported, makes the crate's own
`create_cursor<T>(default_value: T) -> StackCursor<T>`, and
ReactFiberHiddenContext's `CURRENT_TREE_HIDDEN_STACK_CURSOR:
StackCursor<Option<Rc<HiddenContext>>> = create_cursor(None)` was
`createCursor(undefined)`, where upstream's is `createCursor(null)`;
`#[rust_js::nullable]` of the thread-local was refused, as it's no cell of
an `Option`. And the reconciler's `ContextDependency<T>`'s
`memoized_value: Cell<T>`, of `T = Option<&Unknown>`, was left out where
it's made `None`, where upstream writes `memoizedValue: null`;
`#[rust_js::nullable]` of it was refused, as `Cell<T>` is no cell of an
`Option`.

**An `Option` reaching a nullable place is `null`:**

- **`#[rust_js::nullable]` of a field of a type parameter's value**, `T`,
  `Cell<T>` or `RefCell<T>`, is of the `Option` it is where its type
  parameter is one: where the struct is made, `StackCursor { current:
  RefCell::new(None) }` is `{ current: null }`, and where it's assigned or
  its cell is `set`, `replace`d or `take`n, `cursor.current = null`, as a
  nullable field of an `Option` is, by the field's type as it's
  instantiated there. Generic code, of any `T`, stores what it's given as
  it is; its `.d.ts` is `current: T`.
- **`#[rust_js::nullable]` of a thread-local of a generic type made of an
  `Option`**, `StackCursor<Option<T>>`, is of the `None`s its initializer's
  call is given for a type parameter's value, `create_cursor(None)`'s
  `createCursor(null)` (`mir_null_nones`, as a parameter `#[rust_js::nullable(p)]`
  names is): not what's given for a parameter of its own type, `pair(null,
  undefined)`, nor what another call on the way is given, `id(undefined)`.
  Its `.d.ts` is what its type is, `StackCursor<HiddenContext | null |
  undefined>`, as generic code may write a `None` there that's
  `undefined`. Of a thread-local made of no `Option` it's refused, as
  before.

```rust
pub struct StackCursor<T> {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub current: RefCell<T>,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub static CURRENT_TREE_HIDDEN_STACK_CURSOR: StackCursor<Option<Rc<HiddenContext>>> =
        create_cursor(None);
}
```

Its initializer is `createCursor(null)`, made where it's first used, as
a thread-local whose initializer runs the crate's code is (ADR 0037,
amended).

**Alternatives:**

- **A generic function's parameter that reaches a nullable field taken as
  nullable**, `create_cursor`'s `default_value` because it's made into
  `current`: the thread-local's attribute unneeded, but a step through a
  function's body that ADR 0586's first amendment doesn't take of a field's
  items either, and `#[rust_js::nullable(default_value)]` on the function
  already says it where it's wanted.
- **Every generic field's `None` `null`**: what every program's objects
  hold changes, to say what a few of React's do.

### What holds it

- `test/lowering-nullable.test.ts`, "a generic struct's nullable field is
  null for None where it's an Option", run as JS; and
  `test/lowering-module-variables.test.ts`, "a nullable thread-local a
  generic constructor makes is given null for None" and "a nullable
  thread-local of no Option is refused".
- Corpus `nullable_generic_fields`, against native Rust: a cursor pushed
  and popped through generic code, its `None` read back as Rust's.
- Mutations `nullable-generic-field-refused` (`bindings.rs`),
  `nullable-generic-cell-field-refused`,
  `nullable-thread-local-of-options-refused`,
  `nullable-init-of-any-thread-local` (`bindings/nullable.rs`),
  `nullable-init-given-undefined`, `nullable-init-gives-any-parameter` and
  `nullable-init-gives-any-call` (`mir/patterns.rs`).

## Amendment: a nullable thread-local of an `Option` itself, and what one gives (2026-10-26)

React's `ReactFiberCallUserSpace.js` exports each frame's function in DEV,
`null` otherwise:

```js
export const callComponentInDEV = __DEV__
  ? // We use this technique to trick minifiers to preserve the function name.
    (callComponent.react_stack_bottom_frame.bind(callComponent) as any)
  : (null as any);
```

Its port's `thread_local! { pub static CALL_COMPONENT_IN_DEV:
Option<CallComponent> = if __DEV__ { Some(..) } else { None }; }`, which
nothing sets, a `const`, couldn't be `#[rust_js::nullable]`, which was of a
cell of an `Option` alone: its `None` was `undefined`, nine times. And
`ReactProfilerTimer.js`'s `pushComponentEffectErrors`, a nullable
function giving a nullable thread-local's `take()`, read `return old ??
null` where upstream returns it.

- **`#[rust_js::nullable]` is of a thread-local of an `Option` too**,
  whose `init` writes its `None` `null` as a nullable function's result is
  (`null_result`): its `const`'s value, the test's `null` branch,
  `__DEV__ ? f : null`, as preparation writes an exported initializer of
  a test (ADR 0870, amended). Declared `T | null`.
- **What a nullable cell gives, its `take`'s and `replace`'s, is `null` or
  a value already** (`Known::Defined`): each write of it is
  `Storage::NullForNone`'s, and only its module writes a module variable
  (ADR 0270). A nullable place, a nullable function's result too, takes
  it as it is: `const old = componentEffectErrors; componentEffectErrors =
  null; return old;`.

Case A ([0262](0262-when-rust-and-js-disagree.md)), as before: the program
reads `null` and `undefined` alike as `None`. Not yet: the `const` is
`old`, where upstream names it `prevErrors`, a name the Rust doesn't
write. Held by `test/lowering-module-variables.test.ts`, "a nullable
thread-local of an Option is null where it's made, and what it gives is as
it is", its JS, `.d.ts` and values in and out of DEV, and the mutations
`nullable-thread-local-of-option-refused`, `nullable-option-init-undefined`,
`nullable-option-local-declared-undefined` and `nullable-cell-taken-unknown`.

## Amendment: of a `const`

Case ([0262](0262-when-rust-and-js-disagree.md)): A where JS reads it
`== null`, and C where it's `=== null`. React's ReactFiberTransition
exports `export const NoTransition = null`, which its importers compare
`=== NoTransition`. The port's `pub const NO_TRANSITION:
Option<&Transition> = None;` was `export const NoTransition = undefined;`,
and `#[rust_js::nullable]` of it was refused with the thread-local's
message, as a `thread_local!`'s key is a `const` too.

**`#[rust_js::nullable]` on a `const` of an `Option` makes its `None`
`null`**: its value is stored as a nullable place stores it
(`Storage::NullForNone`), `export const NoTransition = null;`, a `Some`'s
value as it is, and its `.d.ts` is `Transition | null`. What the crate's
bodies read of it is the same `None` to Rust either way (ADR 0030); a
body that reads it by its name reads `null`. Of a `const` of anything
else it's refused, as a `const`'s:

```
rust-js: `#[rust_js::nullable]` of a `const` is of one that's an `Option`, whose `None` is then `null`; this one isn't
```

`bindings/nullable.rs`'s `nullable_const_refused` says what's wrong with
either, a thread-local's by its `LocalKey`, a `const`'s otherwise.

### What holds it

- `test/lowering-module-variables.test.ts`, "a nullable const's None is
  null": its JS, its `.d.ts`, and its value imported, and one of a number
  refused.
- Mutations `nullable-const-of-no-option-kept`,
  `nullable-const-taken-for-a-thread-local`,
  `nullable-local-of-no-option-kept` (`bindings/nullable.rs`),
  `nullable-const-made-undefined` (`pipeline.rs`), and
  `nullable-option-local-declared-undefined` (`declarations.rs`), which a
  thread-local of an `Option` shares.

## Amendment: a constant's struct's field

React's reconciler's `CacheContext` is a `ReactContext<Option<Rc<Cache>>>`
whose `_currentValue: Cell<T>` upstream makes `null`. Made in a `const {
.. }` thread-local, `Context { value: Cell::new(None), count:
Cell::new(0.0) }` was `{ count: 0 }`: a constant is written from rustc's
value of it (ADR 0031), which left a nullable field's `None` out as any
`None`, the field's own type's or a generic one's of `T = Option<..>`, the
crate's struct's or a library's alike.

**A constant's `#[rust_js::nullable]` field's `None` is `null`, as where
its struct is made at run time**: of an `Option`, a `Cell` of one, and a
field of a type parameter's value where its constant's type makes it an
`Option`, by the field's instantiated type; an optional one's left out
(ADR 0810). `export const cacheContext = { value: null, count: 0 };`. A
struct with a `RefCell` field is its initializer's (ADR 0863, amended),
whose struct literal says so already.

Case N ([0262](0262-when-rust-and-js-disagree.md)): a read in Rust is the
`Option` either way; a JS reader testing `!== null` sees `null`, as it
does of a struct made at run time.

Tested by `test/lowering-nullable-constants.test.ts`, of the crate's
struct and a library's; its mutations are
`nullable-const-field-undefined` and `optional-const-field-null`.

## Amendment: declared by the aliases its type is written of

Case ([0262](0262-when-rust-and-js-disagree.md)): N, a declaration's
spelling. React's ReactFiberCallUserSpace declares `callComponentInDEV`
by its frame's type, and the port's nullable `thread_local! { pub static
CALL_RENDER_IN_DEV: Option<CallRender> = .. }` was declared
`((value: number) => number) | null`, where a plain one's was
`CallRender | null | undefined`: the `null` union was made of the
`Option`'s type alone, not as its type is written, as a `const`'s and a
function's result are (`as_written`).

**A nullable thread-local, `const` or module variable is declared by the
aliases its type is written of**, as any other is: `export const
callRenderInDev: CallRender | null;`, `export let blockingUpdateTask:
ConsoleTask | null;`.

- `test/lowering-module-variables.test.ts`, "a nullable thread-local of
  an Option is null where it's made, and what it gives is as it is": its
  `.d.ts`, of a thread-local of an `Option` and of a module variable's
  cell.
- Mutation `nullable-local-declared-unaliased` (`declarations.rs`).
