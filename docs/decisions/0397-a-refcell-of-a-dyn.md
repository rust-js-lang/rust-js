# 0397. A `RefCell<dyn T>` is its cell, with its `impl`

Status: Accepted. Extends [0049](0049-traits-and-generics.md) and
[0328](0328-refcell-borrow-counts.md).

Case: C ([0262](0262-when-rust-and-js-disagree.md)): what's changed
through one `Rc` is seen through the other, and each call is the impl
Rust's would call. Where that can't be kept, D: a compile error.

## Context

`Rc<RefCell<dyn Shape>>` is how Rust shares a trait object that changes:
a list of shapes, each borrowed to grow it, with a concrete
`Rc<RefCell<Circle>>` kept beside it. rust-js refused it, as it refuses
any struct whose last field is a `dyn`. rustc's
`dst/dst-coerce-rc.rs` failed on it.

## Decision

**A `RefCell<C>` made a `RefCell<dyn T>`, in an `Rc` or a `Box`, is the
same cell, given the dictionary of `C`'s `T` as its `impl`: the pair a
`dyn` is (ADR 0049), and, as a guard is its cell (ADR 0328), its guard
too.**

```js
const circle = { value: { r: 1 } };
const shapes = [$dynCell(circle, circleShape()), { value: { side: 2 }, impl: squareShape() }];
for (const s of shapes) {
  const receiver = $borrowMut(s, true);
  receiver.impl.grow(receiver, by);
  $unborrow(receiver);
}
```

- **One made where it's coerced** is the literal, `{ value, impl }`.
- **One made before**, which an `Rc` of `C` may share, is
  `$dynCell(cell, impl)`, which gives the cell its `impl` in place: what
  `circle.borrow_mut()` changes, `shapes[0].borrow()` sees.
- **A cell made a `dyn` of a second trait** has one `impl`, both's
  methods: `$dynCell` merges a new trait's into the one the cell has.
  Where two traits, neither the other's supertrait, have a method of one
  name, one `impl` can't hold both, and the coercion is refused,
  `dyn_cell_clash`'s answer, of the type's impls or a generic one's bounds.
- **Its `impl` isn't enumerable**, as its `borrows` isn't: `==` of the
  cell, `$eq`, sees its `value` alone.
- **Dropped**, the cell is the pair it drops through: `cell.impl.$drop(cell.value)`.

## Not yet

- **A counted `Rc<RefCell<dyn T>>`** (ADR 0320), whose `dyn` would be a
  pair around the `Rc`: refused.
- **An upcast**, `RefCell<dyn Sub>` to `RefCell<dyn Base>`, whose
  dictionary would be the cell's `impl`'s `Base()`: refused.

## Why

- **It's the program Rust runs**: the corpus's `dyn_ref_cells.rs` grows
  shapes through the `dyn` and the concrete `Rc`, makes one cell a `dyn`
  of two traits, and compares it; `dyn_ref_cell_supertrait.rs` makes one
  a `dyn` of a trait and its supertrait; `dyn_ref_cell_drop.rs` drops a
  `Box<RefCell<dyn T>>`: each with native Rust.
- **No second representation**: the cell is the pair, so a `dyn`'s call,
  `pair.impl.m(pair.value)`, and a guard's, are the ones every `dyn` and
  `RefCell` has.

## Consequences

- A cell given an `impl` keeps it after its `dyn` is dropped; nothing
  reads it but a `dyn`'s call.
- The diagnostics test holds the refusals: a counted one, two traits of
  one method name, a generic one's two bounds, an upcast.
