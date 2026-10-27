# 0962. A `RefCell` of an object given to JS is the object, and a boxed one is refused

Status: Accepted. Extends [0850](0850-an-rc-is-defined-as-what-it-points-at.md),
an `Rc` `js::Defined` as what it points at, to a `RefCell`, which
[0863](0863-a-refcell-of-an-object-is-the-object.md) makes the object it
holds.

Case ([0262](0262-when-rust-and-js-disagree.md)): N of a `RefCell` that's
the object it holds, which JS shares as Rust does; D, refused, of one
that's its `{ value }` box, which JS would hold in the value's place.

## Context

React's view transitions store an array where JS reads it, then fill it:

```js
applyViewTransitionToHostInstances(current, oldName, className, (current.memoizedState = []), true);
```

The port's natural form shares a cell between the field and the call,
`Rc<RefCell<Vec<_>>>`, stores it in the fiber's `any` field and gives the
call `&RefCell<Vec<_>>` to fill. `js::unknown_of(Rc::clone(&measurements))`
was E0277: a `RefCell` wasn't `js::Defined`, and `js::any_of` refuses a
type that's known. The port filled a `Vec` and stored it after the call,
which reorders what upstream does.

## Decision

**`RefCell<T>` is `js::Defined` where `T` is; a binding call whose type
arguments hold a `RefCell` that isn't the object it holds is refused**
(`std_types::rc::check_given_to_js`, which `check_counted_given_to_js`
was, asking `ref_cells`'s `is_value_cell`).

```rust
let measurements = Rc::new(RefCell::new(Vec::new()));
current.memoized_state.set(Some(js::unknown_of(Rc::clone(&measurements))));
collect(&measurements);
```

```js
const measurements = [];
current.memoizedState = measurements;
collect(measurements);
```

- **A `RefCell` of an array, a map, a set or a struct's object is that
  object** (ADR 0863), which JS holds as the field and the call share it.
  Its borrows, checked or not, are Rust's; what JS does with the object is
  JS's, as with any object it's given.
- **One that's its `{ value }` box**, of a number, an `Option`, or one
  generic code or a `dyn` boxes, is refused where it's given to JS or taken
  from it: "a `RefCell<u32>` given to JS or taken from it .. JS would hold
  its `{ value }` box".

## Alternatives

- **`js::any_of` of a known type**: what it's for is a type parameter's
  value, whose type a program erases (ADR 0909); a known type has its
  `unknown_of`.
- **A `JsObject` struct with an array field, filled through it**: a type
  of rust-js's for what's a `Vec` in Rust.

## What holds it

- `test/lowering-cell-borrows.test.ts`, "a RefCell of an array given to JS
  is the array, and a boxed one is refused", run.
- Mutations: `boxed-refcell-given-to-js`, `value-refcell-given-to-js-refused`
  (`scripts/mutations/lower/std_types/rc.ts`).
