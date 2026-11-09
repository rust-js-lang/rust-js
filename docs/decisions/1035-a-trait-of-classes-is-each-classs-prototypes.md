# 1035. A trait of classes is each class's prototype's

Status: Accepted (rd283); amended by
[1038](1038-a-trait-of-classes-method-that-gives-a-field-is-that-field.md)
(a method that gives a field is that field, read in place in every body,
no prototype method). Amends
[0761](0761-a-rust-js-class-is-a-constructor-function-and-its-prototype.md)
and [0664](0664-a-functions-rest-slice-is-jss-rest-parameter.md): a class implements the crate's trait
each impl of which is a class's, and such a trait's default may take rest
arguments.

Case ([0262](0262-when-rust-and-js-disagree.md)): N, the output's shape:
the same calls, each on the value, as a dictionary's were; what JS can't
give, generic code and a `dyn`, is refused.

## Context

React DOM's ReactDOMRoot gives `createRoot` and `hydrateRoot` two classes
that share their methods:

```js
function ReactDOMRoot(internalRoot) { this._internalRoot = internalRoot; }
ReactDOMHydrationRoot.prototype.render = ReactDOMRoot.prototype.render = function (children) { .. };
ReactDOMHydrationRoot.prototype.unmount = ReactDOMRoot.prototype.unmount = function () { .. };
function ReactDOMHydrationRoot(internalRoot) { this._internalRoot = internalRoot; }
```

The Rust a Rust engineer writes is a trait, upstream's `RootType`, whose
provided `render` and `unmount` both classes take from an `impl RootType
for ..` each. rust-js refused any trait impl of a class with items but
`Display`'s (ADR 0761): a trait's methods were a dictionary's, given the
value, not the prototype's. And `#[rust_js::rest]`, upstream's DEV checks
of `arguments`, was refused on a trait's method (ADR 0664). Writing the
methods twice, an inherent impl each, copies a body; `with_this` functions
set on both prototypes lose the `.d.ts` and read as dynamic (gap rd283,
P1: it held ReactDOMRoot and so ReactDOMClient).

## Decision

**A trait of classes, the crate's trait with items each impl of which is
a `#[rust_js::class]`'s, is each class's prototype's** (`classes::
is_class_trait`):

- **Each method an impl writes is its class's prototype method**, as an
  inherent one is, named as the trait's, and **each default it takes is
  compiled for that class with `Self` known**, its MIR instantiated, a
  `function` reading `this` (`classes::prototype_defaults`, of the impl's
  dictionary lowered, which is no `const`). A call in it of a method its impl gives
  only a reference to a field with, `fn internal_root(&self) -> &RefCell
  <..> { &self.internal_root }`, is that field borrowed in place
  (`mir/fields_read.rs`'s `Mir::field_given` and `Mir::fields_read`).
- **Where two classes of a module print a default alike, one function is
  assigned to both**, the later first, as upstream chains them:
  `ReactDOMHydrationRoot.prototype.render = ReactDOMRoot.prototype.render
  = function (children, ...args) { .. }` (`classes::take_default`,
  `js::Class::shared`).
- **A Rust call of its method is a call on the value**, `root.render(x)`
  (`traits/calls.rs`'s `trait_call`), and each class's `.d.ts` declares the trait's
  methods.
- **A default may take rest arguments**, `#[rust_js::rest] args: &[T]`,
  JS's `...args`, which each class's call spreads (ADR 0664, amended).

Refused, each saying what to write: such a trait bounding a type
parameter or a `dyn`'s (no dictionary is made for generic code; a
`T: RootType` caller would need one for a class); of type or const
parameters, of supertraits, or of an item that's no method of `&self` or
`&mut self`; a method named as one of a class's fields, which the
instance's own property hides; an impl outside its class's module, where
its prototype is; and an impl's own method of a default that takes rest
arguments, which it would be given spread. A trait a struct that's no
class implements too is a dictionary's, and its class's impl refused as
before.

## Alternatives

- **A dictionary of the class's prototype**, generic callers given one
  `{ render: (self, ..) => self.render(..) }`: a second form of each
  call, for callers the port has none of; refused until one does.
- **One copy of a default for every class, `this` dispatching the rest**:
  a default whose `Self` calls differ per class, an impl's own method,
  can't be one function; compiled per class, alike ones shared, it's
  upstream's.
- **Each class's own copy**: correct, but not upstream's one function.

## Consequences

- The port writes React DOM's `RootType` as a trait with `internal_root`,
  `render` and `unmount`, each root `impl RootType for ..` with
  `internal_root` alone.
- ~~Not yet upstream's: the impl's `internal_root` stays a prototype
  method, a cell's handle, and a default's borrow of the field is
  checked.~~ Done by ADR 1038: the method is the field, read in place,
  `this._internalRoot`, its borrows the field's own.
- Held by `test/classes-traits.test.ts`: the shared and own methods, a
  call, rest arguments, the `.d.ts`, run in JS; each refusal. The corpus's
  `class_trait_methods`, against native Rust. Mutations
  `class-trait-call-through-dictionary` (`traits/calls.ts`),
  `class-defaults-unshared`, `class-defaults-shared-unlike`,
  `class-default-self-unread`, `class-trait-of-any-impls`,
  `class-trait-bound-allowed`, `class-trait-dyn-allowed`,
  `class-trait-rest-overridden`, `class-trait-method-hidden`
  (`classes.ts`), `class-default-fields-unread` (`traits/impls.ts`),
  `field-read-of-the-reference` (`mir/fields_read.ts`),
  `rest-of-class-trait-refused` (`analysis/validation.ts`) and
  `class-trait-methods-undeclared` (`declarations/functions.ts`). No mutation of the
  impl's module check: no test makes a class in one module and its impl
  in another yet.

## Amendment: a library's trait of classes is one in its consumer (#1073)

Case ([0262](0262-when-rust-and-js-disagree.md)): results change, so
Rust's. `is_class_trait` asked `trait_id.is_local()`, so a library's trait
of classes was a dictionary's in its consumer. Each call of a method of
the library's class compiled the consumer's own copy of it and of a
dictionary the library never made (`rootRender_label(root)`); a library's
class whose trait of classes has a `then` was adopted where a consumer
awaited one, its `.await` never ending (#1071's hazard across crates); a
method that only gives its field, with no JS of its own (rd304), was
compiled again; a consumer's `dyn` of the trait read that dictionary; and
a consumer's class's impl of it was refused.

A trait is now one of classes where each impl its own crate has is a
class's: the impls its crate's compile saw, as a trait's impls are in its
crate or in one that uses it, so a library's consumer asks the library's
decision of the same impls, read of its metadata, which the library's JS
is checked to be of (`Foreign::check`). No manifest fact carries it: one
would be a second answer of what the metadata already decides, needed by
the classes owner's callers, which ask it of a `TyCtxt` alone. So:

- **A consumer's call of its method is a call on the value**,
  `root.label()`, as the library's own calls are; a method that only gives
  its field is that field, read in place, of the library's MIR
  (`classes::field_accessor`, a body of nothing but `_0 = &((*_1).slot)`).
- **A consumer's class may implement it**, each method its prototype's;
  a default its library's MIR, compiled for the class where it's asked
  (ADR 0914), `Leaf.prototype.render = function (times) { .. }`.
- **What the library refuses of it, its consumer refuses too**: a `dyn`
  of it, a type parameter it bounds; and an impl of it for what's no
  class, whose calls, the library's defaults' `this.label()` among them,
  are on a prototype it hasn't: "a `#[rust_js::class]` alone implements
  `dep::Render`, a library's trait of classes".
- **A library's class's trait-of-classes methods are asked where its
  value is awaited** (`classes::has_method_named`), its `then` among them.

Held by `test/library-classes.test.ts` (the calls, a consumer's class's
default, a `then` across crates, each refusal), the pairs
`library_trait_of_classes` and `library_trait_of_classes_given_fields`
(a method that sets a field, calls one, drops a value, gives another's
field, one through a reference, or a `&mut`, before or as it gives a
field), against native Rust, and React's port, whose JS is unchanged.
Mutations `class-trait-of-the-crate-alone`,
`class-trait-of-every-crates-impls`, `class-trait-library-impls-unseen`,
`library-class-trait-impl-of-no-class-allowed` (`lower/classes.ts`),
`library-class-default-through-its-library`,
`library-class-default-unasked` (`lower/traits/impls.ts`),
`library-field-accessor-unread`, `library-field-accessor-past-its-end`,
`library-field-accessor-after-statements`,
`library-field-accessor-of-a-mut` and
`library-field-accessor-of-another-parameter`
(`lower/classes/accessors.ts`).
