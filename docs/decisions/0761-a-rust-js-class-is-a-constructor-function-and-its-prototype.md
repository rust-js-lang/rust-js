# 0761. A `#[rust_js::class]` is a constructor function and its prototype

Status: Accepted, by the user's decision of 2026-10-24. Extends
[0047](0047-methods.md), whose object of functions stays every other
struct's impl. Amended: its struct's comments, its `///` doc, are its
constructor function's (below). Amended (2026-10-26, the user's
decision): a constructor of statements makes its instance in a variable,
`this`; the struct's `#[rust_js::name]` names the class, declared where
its constructor is; a `PartialEq` and a `Hash` of its address are
allowed. Extended by
[0911](0911-a-bindings-class-is-the-global-typescript-names.md):
`js::class::<T>()` of a binding's type is the global class TypeScript
names it. Extended by [0943](0943-an-object-of-methods-reading-this.md): a
`#[rust_js::object]`'s methods are its literal's own, not a prototype's.
Amended by [1009](1009-a-constructor-is-held-as-a-typed-class.md):
`js::Class` takes its constructor's signature, `Class<fn(A, B) -> T>`,
`js::class::<T>()` the untyped one it defaults to.

Case ([0262](0262-when-rust-and-js-disagree.md)): A for what's kept, a
value's fields and its methods' meaning; and a compile error where Rust's
can't be kept: a method named as a field, which `this.name` would hide, a
struct literal of the class outside its `new`, which would be no
instance, a trait's methods, which would be a dictionary's.

## Context

React's packages/react/src/ReactBaseClasses.js is a class written as JS
wrote one before `class`, and apps write `class X extends
React.Component`:

```js
function Component(props, context, updater) {
  this.props = props;
  this.context = context;
  this.refs = emptyObject;
  this.updater = updater || ReactNoopUpdateQueue;
}
Component.prototype.isReactComponent = {};
Component.prototype.setState = function (partialState, callback) { .. };
```

rust-js had no way to write it. A struct's impl is an object of functions
given the value (ADR 0047), `Component.setState(component, ..)`, which a
JS subclass can't call as `this.setState(..)`, and `Component.new(..)` is
no constructor `super(props)` can run. Its port wrote `fn component(..)`
with its body commented out and the prototype set by `js::set` of
`js::get(js::unknown(&component), "prototype")`.

## Decision

**A struct and its inherent impl, each marked `#[cfg_attr(rust_js,
rust_js::class)]`, are a JS class**: a constructor function and its
prototype (`classes.rs`, `js::Namespace::class`, printed by
`to_oxc::class`).

- **Its fields are `this`'s; `fn new(..) -> Self` is the constructor
  function**, `export function Component(props, context, updater) {
  this.props = props; .. }`: each `return Self { .. }` of its body is each
  field set on `this`, in the order written, its comment before it, and
  `return;` where it's not the last.
- **A method of `&self` or `&mut self` is the prototype's**,
  `Component.prototype.setState = function (partialState, callback) { ..
  };`: a `function`, so `this` is what it's called on, and its `self` is
  `this`.
- **An associated `const` is the prototype's property**,
  `Component.prototype.isReactComponent = {};`, camelCase, as a method is
  (`classes::const_name`); always written, as it's the prototype's,
  whoever reads it.
- **A function of no `self` is the constructor's own**, `Component.from =
  function (..) { .. };`.
- **A Rust call is JS's**: `Component::new(..)` is `new Component(..)`,
  `c.set_state(..)` `c.setState(..)` (`FnCx::crate_called`); a method or
  the constructor as a value is the arrow of its call, `(a, b) =>
  a.forceUpdate(b)`.
- **`js::class::<T>()` is the constructor function itself**, `Component`,
  a `&js::Class`, whose `prototype()` and `set_prototype(..)` are JS's
  `C.prototype`: what a prototype chain is written with.
- **Its declaration is a class's**, `export declare class Component {
  constructor(..); props?: unknown; setState(..): void; }`, each `pub`
  member's, so a TypeScript `class X extends Component` type-checks.

### Its struct's doc is its constructor function's

*Amendment.* The struct's `///` doc was its `.d.ts` class's alone, and
gone from the JS: a struct is a type, which has no JS, and the
constructor function is `new`'s, in the impl. **The comments before the
struct, its doc, are before the constructor function**, then `new`'s
own, each as written (`js::Class::declared`, from the lowering's
`classes::class_of`; attached by `to_oxc::declared_class`), as React has
`/** Base class helpers .. */` above `function Component`:

```js
/** Base class helpers for the updating state of a component. */
export function Component(props, context, updater) {
```

A comment inside the struct, a field's, has no JS still: no field is
declared but by the constructor's assignments, which are `new`'s
statements.

### PureComponent's prototype chain

Rust has no inheritance, so a port writes React's own steps, as React
writes them, with `js::class`:

```rust
js::on_load! {
    js::class::<ComponentDummy>().set_prototype(js::class::<Component>().prototype());
    let pure_component_prototype = js::unknown_of(ComponentDummy::new());
    js::class::<PureComponent>().set_prototype(pure_component_prototype);
    js::set(pure_component_prototype, "constructor", js::class::<PureComponent>());
    assign(pure_component_prototype, (js::class::<Component>().prototype(),));
    js::set(pure_component_prototype, "isPureReactComponent", true);
}
```

```js
ComponentDummy.prototype = Component.prototype;
const pureComponentPrototype = new ComponentDummy();
PureComponent.prototype = pureComponentPrototype;
pureComponentPrototype.constructor = PureComponent;
Object.assign(pureComponentPrototype, Component.prototype);
pureComponentPrototype.isPureReactComponent = true;
```

`ComponentDummy` and `PureComponent` are classes of their own, each its
fields and `new`, as React's are each a function of their own.

### Refused

Each a compile error that says what to write (`classes::reject_misuse`):

- a tuple or unit struct: a class's fields are `this`'s, named;
- a type or const parameter: one prototype is every value's;
- a class of no `fn new(..) -> Self`, or one that gives other than `Self`;
- the attribute on the struct or its impl alone;
- a struct literal of it outside its `new`: a plain object, no instance;
- a trait impl of it with items, a `derive(Clone)` or a `Drop` among
  them: a trait's methods are a dictionary's, given the value, and a
  clone would be a plain object; a marker's, `unsafe impl js::Defined`,
  is fine;
- a method of `self` by value: its receiver is `this`;
- a method or a constant named as a field, `count` and `count()`:
  `this.count` would hide the prototype's;
- `js::class::<T>()` of a `T` that's no class;
- a constructor whose body gives what's no struct literal.

### One way, beside ADR 0047's

This is the one way to make a JS class. ADR 0047's object of functions
stays the shape of every other struct's impl, and that isn't two ways to
do one job: each is the JS shape a caller sees of a type, a plain object
of data, which a JSON value is too, and its functions, or an instance of
a class, which `instanceof` and `extends` see. Which one a type is is
where Rust meets JS, so it's an attribute, as a binding's
`#[rust_js::link_name]` is; the Rust of either is the same Rust.

## Why

- **It's React's JS**: `function Component(props, context, updater) {
  this.props = props; .. }`, its prototype's `setState` a `function` of
  `this`, read line by line against upstream's.
- **JS subclasses work**: `class X extends Component` runs the
  constructor through `super`, and the prototype's methods see the
  subclass's `this`.
- **The Rust is the Rust**: a struct, its `new`, its `&self` methods,
  called as Rust calls them.

## Alternatives

- **An ES `class`**, `export class Component { constructor(..) { .. }
  setState(..) { .. } }`: not React's JS, whose prototype a port sets as
  a property; and a class's constructor can't be called without `new`,
  which React's `ComponentDummy` and old callers are.
- **Every struct with methods a class**: ADR 0047's plain objects would
  stop being JSON's, and a copy would need its prototype.
- **`#[rust_js::class(extends = Component)]`**: a second spelling of a
  prototype chain JS already writes in three statements, with nothing
  Rust checks.
- **The `.d.ts` an interface and a constructor's type**: TypeScript's
  `extends` takes a class declaration as it is.

## Consequences

- An instance of a class is no plain object: `{ ..c }` copies its own
  fields, not its prototype's methods, so a class implements no `Clone`.
- A read of the class's constant in Rust reads the prototype's,
  `Counter.prototype.limit`, which a JS caller may change: case B, rare
  and harmless, as a port's constant is React's prototype property.
- Its JS is in two parts, its prototype set where its module's types are
  and its `on_load!` where its statements are: `ComponentDummy` and
  `PureComponent` are declared before `ComponentDummy.prototype = ..`,
  where React's interleaves them.

- A constructor's early `return Self { .. }` is its fields set and
  `return;`, as JS's constructor function returns: the corpus's share of
  `return;` in docs/readability.txt grew, 9.3 to 9.4 in 10,000 lines, of
  `class_methods`'s.

## What holds it

- `test/classes.test.ts`, run in Node: React's `Component`,
  `ComponentDummy` and `PureComponent` as a port writes them, their JS and
  `.d.ts`, a JS `class Counter extends Component` calling `this.setState`
  and `this.forceUpdate`, a `class Pure extends PureComponent`,
  `instanceof`, `isReactComponent`, `isPureReactComponent`, the
  `constructor` fix-up, `setState(5)`'s `Error`, Rust's `new` and method
  calls and a method as a value; and each refusal's message.
- Corpus case `class_methods`, against native Rust: a constructor's early
  `return`, `&self` and `&mut self` methods, a constant, a static, a
  method as a value.
- `test/classes.test.ts`, "a class struct's doc is its constructor
  function's": the struct's doc, then `new`'s, before `function
  Component`; a field's comment and a plain struct's doc nowhere.
- Mutations `class-doc-unattached`, `class-doc-unstarted`
  (`to_oxc.rs`), `class-declared-nowhere` (`lower/pipeline.rs`).
- Mutations `class-unmarked`, `class-method-a-static`,
  `class-self-not-this`, `class-constructor-returns`,
  `class-const-screaming`, `class-trait-allowed`, `class-literal-allowed`,
  `class-by-value-allowed`, `class-field-method-clash-allowed`,
  `class-generic-allowed`, `class-unmade-allowed`,
  `class-impl-unmarked-allowed` (`lower/classes.rs`),
  `class-constructor-called`, `class-method-called-as-a-function`
  (`lower/mir/calls.rs`), `class-const-not-the-prototypes`
  (`lower/items.rs`), `class-method-on-the-constructor`,
  `class-constructor-unexported` (`to_oxc/statements.rs`),
  `js-class-of-the-constructor-call` (`lower/binding_calls.rs`).

## Amendment: a method's `self` is `this` before tidy (#344)

A method's `self`, named after its type, `point` of a `Point`'s, was made
`this` after tidy, every variable of its name with it. Tidy's shadows
(ADR 0650) take a `let` of that name, read no more after it, for the
parameter: `let point = Point::new(self.x + by)` is `point = Point.new(..)`
of a plain method, which is right, and of a class's method was `this =
new Point(..)`, which rust-js couldn't print (#344; trekhleb/javascript-algorithms'
ComplexNumber, `let complex_number = self.to_complex_number(number)`).
**A class's method's `self` is `this` before tidy**
(`classes::receiver_as_this`), so a `let` of its name is a variable of
its own, `const point = new Point(this.x + by)`. Held by
`test/corpus/class_local_named_as_self.rs` and the mutations
`class-receiver-left-to-tidy` and `class-self-not-this`.

## Amendment: a class may be its module's default (#345)

trekhleb/javascript-algorithms' ComplexNumber is `export default class
ComplexNumber`, and `js::export_default!(ComplexNumber)` was refused: the
naming form read Rust's value namespace alone, where a braced struct
isn't. **A class is a value, its constructor function, so it may be its
module's default**: `function ComplexNumber(parts) { .. } .. export
default ComplexNumber;`, exported by no name of its own, as a default
function is (ADR 0611); declared `declare class ComplexNumber { .. }
export default ComplexNumber;`; and a module of the crate that uses it
imports the default, `import ComplexNumber from "../complex_number/mod.js"`,
and its type, `import type ComplexNumber from ..`. Only a class: any
other struct is a type, no value to export. Held by `classes.test.ts`'s
"a class is its module's default export" and the mutations
`default-class-refused`, `default-class-exported-by-name`,
`default-class-imported-by-name`, `default-class-declared-const` and
`default-class-type-imported-by-name`.

## Amendment: its `bool` methods' calls stay `bool`s (#355)

With a method's `self` `this` before tidy (#344), a call of its own
`bool` method, `self.less_than(a, b) || self.equal(a, b)` of
trekhleb/javascript-algorithms' Comparator, read `!!this.lessThan(a, b) ||
..`: the `bool` paths lowering records are of the receiver's name,
`comparator.lessThan()`, and tidy read no path through `this`.
`receiver_as_this` renames them `this.lessThan()`, and `field_path` reads
`this` as a variable's name. Held by `test/corpus/class_bool_methods.rs`
and the mutations `receiver-booleans-unrenamed` and
`field-path-not-of-this`. A call of a function that gives a `bool` is
known one where it's lowered now (`Known::Boolean`, ADR 0953, amended),
so the path through `this` holds a read of the class's own `bool` field,
`this.open || this.below(x)`, the case's `openOrBelow` (2026-10-30).

## Amendment: a handle on `this`'s field reads the instance (#359)

Case C ([0262](0262-when-rust-and-js-disagree.md)): results changed. With
a method's `self` `this` before tidy (#344), `self.reads.set(..)` of a
`Cell` field kept its handle, `({ get value() { return this.reads; }, ..
}).value = ..`, whose `this` is the handle: the field was never set.
Tidy's `read_handles` takes a handle's place where it's rooted at `this`,
as it does at a variable, so the method reads `this.reads = ..` again.
A handle the method keeps, one it hands on, `bump(&self.reads)`, reads
the instance by a name of its class's, `const counted = this;`, outside
its accessors (`this_outside_accessors`). Held by `classes.test.ts`'s "a
class's method sets its Cell fields on this",
`test/corpus/class_cell_fields.rs` and the mutations
`class-handle-this-unnamed` and `read-handle-this-rooted-kept`.

## Amendment: a getter is the prototype's (#357)

Decided by the user (2026-10-26). trekhleb/javascript-algorithms'
`QueenPosition` has `get leftDiagonal()`, which its test reads as
`position.leftDiagonal`; a class's method was always the prototype's
function. **A class's method marked `#[cfg_attr(rust_js,
rust_js::getter)]` is its prototype's getter**, written in the
constructor-function style of this ADR, `Object.defineProperty(
QueenPosition.prototype, "leftDiagonal", { get() { .. } })`, and declared
`get leftDiagonal(): number`. A Rust call of it, `p.left_diagonal()`, is
its read, `p.leftDiagonal`; as a value it's the arrow of that read, as a
method's is the arrow of its call. Rust has no property syntax, so the
attribute, where Rust meets JS, says which method is one: the one way to
make a getter. Only a method of `&self` alone, not `async`, giving a
value, is one; `&mut self`, arguments, no `self` or a struct that's no
class are refused, saying what to write. A setter would be a
`rust_js::setter` of its own when a port needs one. Held by
`classes.test.ts`'s "a class's getter is its prototype's" and "what a
getter is refused says what to write", `test/corpus/class_getters.rs`, and
the mutations `getter-a-method`, `getter-of-no-class-allowed`,
`getter-of-any-shape-allowed`, `getter-called`, `getter-value-unread` and
`getter-printed-as-method`.

## Amendment: its `Display` is its prototype's `toString` (#358)

Decided by the user (2026-10-26). trekhleb/javascript-algorithms'
`QueenPosition` has `toString() { return `${this.rowIndex},${this.columnIndex}`; }`,
which its test calls; a class's trait impl with items was refused, so
`impl fmt::Display` couldn't be written. **A class's `impl Display` is its
prototype's `toString`**, its `fmt` a method (`classes::shown_class`)
named `toString` on its class, no dictionary of its own. Case A
([0262](0262-when-rust-and-js-disagree.md)): JS's text of the value,
`${p}`, calls `toString` with nothing, which is what Rust's `{p}` writes
where the crate gives no `Formatter` options, so `{p}` and `p.to_string()`
are `${p}`, and a `T: Display` it's given shows it by `String`. Where the
crate's writers take options (ADR 0137), `{p:>6}` is `p.toString({ width:
6, .. })`, and a plain one `p.toString()`, as its other writers are
given them. It's declared `toString(): string`. Only `Display`: another
trait's methods would still be a dictionary's. Held by `classes.test.ts`'s
"a class's Display is its prototype's toString",
`test/corpus/class_display.rs`, `test/corpus/class_to_string.rs`, and the
mutations `class-display-no-member`, `class-display-refused`,
`class-display-owned-apart`, `class-display-named-fmt`,
`class-display-called`, `class-display-template-arrow`,
`class-writer-called-by-name`, `class-to-string-given-undefined`,
`class-display-dictionary-impl`, `class-display-dictionary-emitted` and
`class-to-string-undeclared`.

## Amendment: a constructor of statements, a class's name, its home and its identity (2026-10-26)

React's ReactFiber.js makes a fiber with a constructor function of
statements, of a struct ReactInternalTypes declares:

```js
function FiberNode(tag, pendingProps, key, mode) {
  this.tag = tag;
  ..
  if (enableProfilerTimer) {
    this.actualDuration = -0;
    ..
  }
  if (__DEV__) {
    this._debugOwner = null;
    ..
    Object.preventExtensions(this);
  }
}
```

A class's `new` was one struct literal, so it couldn't set a field on a
flag nor call anything on `this`; a `None` field was `this.x =
undefined;`; the class was named as its struct, `Fiber`; a struct of
another module had its declaration nowhere, `createFiber(): any`; and a
fiber's `PartialEq` of `ptr::eq` and `Hash` of `ptr::hash`, which a `Set`
of fibers needs, were refused as a trait's methods. The user decided
(2026-10-26) the form to build: an extension of the class form, one way.

- **A constructor of statements makes its instance in a variable**, the
  one its last statement returns, made of a struct literal, as Rust
  writes a value it changes before giving it: `let fiber = Fiber { .. };
  if on { fiber.actual_duration.set(Some(-0.0)); } fiber`. That value is
  `this` (`classes::instance_as_this`): its literal's fields are set on
  `this` where it's made, each read of it is `this`, an early `return
  fiber` is `return;`, and a handle on its field is on the instance, named
  outside the handle's accessors, as a method's is (#359). Set again,
  `fiber = Fiber { .. }`, it's refused, as `this` can't be.
- **A `None` field is left out**, `this.actualDuration` set only where the
  flag sets it, as an object's is (ADR 0280): `js::undefined_keys`, moved
  from preparation to the owned AST so the constructor's lowering and
  preparation ask one rule.
- **The struct's `#[rust_js::name]` names the class**, its constructor
  function, its `new` calls and its declaration, `FiberNode` of `Fiber`,
  as a binding's class is named (`bindings::class_name`).
- **A class is where its constructor function is** (`classes::home`):
  its declaration is in that module's `.d.ts`, imported from there, though
  its struct is another module's.
- **A `PartialEq` and a `Hash` of its address**, `ptr::eq` and
  `ptr::hash`, of a class equal only to itself (ADR 0864), are allowed:
  `==` is `===` and a `HashSet` a `Set` of the objects, with no JS of
  their own. The check runs where the crate's trait impls are known.

Case ([0262](0262-when-rust-and-js-disagree.md)): A; a `None` field left
out is ADR 0280's A, what reads it reads `undefined` either way.

The corpus's share of `return;` in docs/readability.txt grew, 10.2 to
10.3 in 10,000 lines: `class_constructor_statements`'s early `return
fiber`, which a constructor function returns as `return;`, as above.

Not decided here: React's `createFiberImplObject`, the object form
`enableObjectFiber` picks, is a struct literal of the class outside its
`new`, still refused.

Held by `test/classes.test.ts`, "a constructor's statements set this, of
a class named apart from its struct, in another module", "a class equal
only to itself compares by ===" and the refusals' "class-instance-set-
again", run in Node; corpus `class_constructor_statements`, against
native Rust; and mutations `class-instance-not-this`,
`class-instance-set-again-allowed`, `class-constructor-none-set`,
`class-constructor-handle-on-this`, `class-instance-return-kept`,
`class-identity-impls-refused`, `class-home-the-structs`
(`lower/classes.rs`), `class-declared-by-its-struct-name`,
`class-of-another-modules-struct-undeclared` (`lower/declarations.rs`) and
`class-named-by-its-struct` (`lower/analysis/naming.rs`).

## Amendment (2026-10-28): a typed `js::Class`

[1009](1009-a-constructor-is-held-as-a-typed-class.md): `js::Class` is
`Class<S = AnyConstructor>`, typed by a constructor's signature where
`Class::of(f)` holds a function the program writes, `Class<fn(A, B) ->
T>`, which `js::new(class, (a, b))` is given what it takes of and makes a
`&'static T` of, and which its declaration is, `new (a: A, b: B) => T`.
`js::class::<T>()` is the untyped `Class`, as before: a class's
constructor's own call is `T::new(..)`.
