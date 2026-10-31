# 1038. A trait of classes' method that gives a field is that field

Status: Accepted (rd304). Amends
[1035](1035-a-trait-of-classes-is-each-classs-prototypes.md) (such a
method is no prototype method, and is read in place in every body, not
only a default) and
[0863](0863-a-refcell-of-an-object-is-the-object.md) (a call of it is
the field, no lending of it, for whether the field's borrows can meet).

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the output's shape:
each call reads the field the method gives, which is what the method
does; every borrow keeps Rust's check where one may meet another.

## Context

React DOM's roots share `render` and `unmount` through a trait of
classes (ADR 1035), whose impls give the field they work on:

```rust
pub trait RootType {
    fn internal_root(&self) -> &RefCell<Option<Rc<FiberRoot>>>;

    fn render(&self, children: ReactNodeList) {
        let Some(root) = self.internal_root().borrow().clone() else {
            panic!("Cannot update an unmounted root.");
        };
        ..
    }
}

impl RootType for ReactDOMRoot {
    fn internal_root(&self) -> &RefCell<Option<Rc<FiberRoot>>> {
        &self.internal_root
    }
}
```

Upstream reads `const root = this._internalRoot; if (root === null) {
throw .. }`. rust-js gave each class an `internalRoot` prototype method
returning a handle on the field, `$fieldCell(this, "_internalRoot",
true)`, and checked each borrow of it, `$borrow(..)`: the method's own
`&self.internal_root` lent the field (ADR 0863's quiet fields count a
field lent), and a default's call of it, of a `Self` not known, may have
run anything, so a borrow held across it was counted. Where a library
borrows the cell's type unchecked, that counted borrow was refused
outright, "held while code rust-js can't see runs" (gap rd304, P1: it
held ReactDOMRoot, and so ReactDOMClient).

## Decision

**An impl's method of a trait of classes whose body only gives a
reference to a field of its `&self`, `&self.f` or `&self.a.b`, is that
field** (`classes::field_accessor`, of its HIR and types: the struct's
own fields, through `self`'s reference alone):

- **Every call of it is the field borrowed in place**, in every body
  (`Mir::read_fields`, run over each body as its MIR is collected, and
  over a default's once its `Self` is known), so it's no prototype
  method, nor declared: `this._internalRoot`, as the class's own methods
  read it.
- **For whether a field's borrows can meet (ADR 0863), a call of it is
  the field**: its own `&self.f` lends nothing; a borrow through its
  call is the field's borrow, counted where it's held while code that
  may borrow it runs; and its call held elsewhere, `fill(holder.slot())`,
  lends the field (`classes::fields_given`, in `drops/ref_cells.rs`).
- **A default's call of a trait of classes' method, of a `Self` not
  known, runs what one of its classes' does** (`classes::
  implementations`, `cell_borrows::Called::Bodies`): its impls are the
  crate's classes', no others. So a borrow held across
  `self.internal_root()` is held across no code, and one held across
  `self.probe()` is counted where a class's `probe` borrows its cell.
- **Named as the field it gives**, `fn items(&self) -> &RefCell<..> {
  &self.items }`, it's allowed: no method the field would hide.

Refused, saying what to write: such a method held as a function, `let
given = <Root as RootType>::internal_root;`, which JS has no function
for: call it.

## Alternatives

- **Keep the method as a prototype getter of a handle, and count its
  callers' borrows as the field's**: correct, but a method upstream
  hasn't, and a handle JS has no use for.
- **Count only a default's borrows as the field's, keeping the method**:
  the method's own lending keeps the field checked, `$fieldCell`, as
  before.

## Consequences

- React DOM's `RootType` reads `this._internalRoot` in `render` and
  `unmount`, as upstream does; the rd304 refusal is gone.
- A JS caller of a class has no `internalRoot()` of the trait; it reads
  the field, declared where it's public.
- Held by `test/classes-traits.test.ts` (the read in place, no method or
  declaration, the refusal of one held) and the corpus's
  `class_trait_field_borrows` (momentary and counted borrows, a counted
  property cell, a call outside a default, a class whose method gives
  what an `Rc` holds), `class_trait_field_lent` (one lent elsewhere
  panics as std's does) and `class_trait_field_held` (a borrow held
  across a default's call of another trait method), against native
  Rust. Mutations `field-accessor-reborrow-refused`,
  `field-accessor-of-any-path`, `field-accessor-held-allowed`,
  `field-accessor-name-clash-refused`, `class-trait-unknown-self-any`
  (`classes.ts`), `class-trait-call-runs-anything`,
  `class-trait-call-asks-nothing` (`analysis/cell_borrows.ts`),
  `field-accessor-lends-its-field`, `field-accessor-call-lends-nothing`,
  `field-accessor-borrow-of-no-field` (`drops/ref_cells.ts`),
  `class-trait-field-calls-unread` (`analysis.ts`),
  `class-trait-field-method-printed` (`pipeline/reached.ts`) and
  `class-trait-field-method-declared` (`declarations/functions.ts`).
