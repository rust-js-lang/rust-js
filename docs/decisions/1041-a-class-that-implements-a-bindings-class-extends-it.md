# 1041. A class that implements a binding's class extends it, an ES class

Status: Accepted, by the user's decision of 2026-11-01 to add class
components (rd331). Amends
[0761](0761-a-rust-js-class-is-a-constructor-function-and-its-prototype.md),
whose Alternatives rejected an ES class and `#[rust_js::class(extends =
..)]`: a class that extends a binding's is an ES class, said by the
binding's trait it implements, not an attribute. Amends
[0347](0347-react-coverage.md)'s list of what the react crate leaves out:
it binds `Component`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N for the class, which
is React's to construct and call as JS's is, and whose methods' bodies are
Rust's as any method's; A for a field React's instance gains after, which
is `undefined` until set, an `Option`'s `None` (ADR 0280).

## Context

react-art's ReactART.js writes its components as classes, as a library
that React or a JS caller constructs does:

```js
class Surface extends React.Component {
  componentDidMount() {
    const {height, width} = this.props;
    this._surface = Mode.Surface(+width, +height, this._tagRef);
    ..
  }
  componentWillUnmount() { .. }
  render() { .. }
}

class Text extends React.Component {
  constructor(props) {
    super(props);
    ..
  }
  render() { .. }
}
```

rust-js had no way to write one. A `#[rust_js::class]` is a constructor
function and its prototype (ADR 0761), which extends nothing but by hand,
`js::class::<T>().set_prototype(..)` in `js::on_load!`, has no
`super(props)` to run `React.Component` on `this`, and no ES class's
`extends`; and the react crate bound no class components, by the user's
choice of 2026-10-10. On 2026-11-01 the user decided to add them: a
struct whose class extends a binding's class prints upstream's ES class.

## Decision

**A `#[cfg_attr(rust_js, rust_js::class)]` struct that implements a
binding's class is an ES class that extends it.** The react crate's
`Component<P = (), S = ()>` is one, a trait whose `#[rust_js::types]`,
`react#Component<P, S>`, names the class (`classes::base_class`), as a
binding's type names the class `instanceof` tests it by (ADR 0911):

```rust
use std::cell::Cell;

use react::{Component, JSX, jsx};

pub struct CounterProps {
    pub label: &'static str,
}

/// Says what React does with it.
#[cfg_attr(rust_js, rust_js::class)]
pub struct Counter {
    _mounts: Cell<Option<f64>>,
}

impl Component<CounterProps> for Counter {
    fn component_did_mount(self: &'static Self) {
        self._mounts.set(Some(self._mounts.get().unwrap_or(0.0) + 1.0));
        println!("mount {}", self.props().label);
    }

    fn component_did_update(self: &'static Self, prev_props: &CounterProps, _prev_state: &()) {
        println!("update {} to {}", prev_props.label, self.props().label);
    }

    fn render(self: &'static Self) -> JSX::Element {
        let props = self.props();
        jsx! { <p>{props.label}</p> }
    }
}
```

```jsx
import { Component } from "react";

/** Says what React does with it. */
export class Counter extends Component {
  componentDidMount() {
    this._mounts = (this._mounts ?? 0) + 1;
    console.log(`mount ${this.props.label}`);
  }

  componentDidUpdate(prevProps, prevState) {
    console.log(`update ${prevProps.label} to ${this.props.label}`);
  }

  render() {
    const props = this.props;
    return <p>{props.label}</p>;
  }
}
```

- **Each method its impl writes is its class's**, in its order
  (`classes/subclasses.rs`' `subclass_impl_class`, `trait_member_class`): `render` and
  each lifecycle method, named as the trait's is. **One it doesn't write
  its class doesn't have**: the trait's default has no JS, so React does
  what it does of none, as it tests `typeof instance.componentDidMount`,
  and a Rust call of it is refused where a call of what has no JS is.
- **What the class it extends has is its**: a method of the trait that's
  a binding, `props` as `get props`, `this.props`, typed `&P`, and
  `force_update`, `this.forceUpdate()`. An impl that writes one is
  refused, as its own would hide React's.
- **React constructs it**: a class of no `fn new` is made by the class it
  extends, and each field is what the instance gains after, `undefined`
  until set: an `Option`, or a `Cell` or `RefCell` of one; any other is
  refused (`starts_undefined`).
- **Or it writes its constructor**, `fn new(props: &P) -> Self` in its
  `#[rust_js::class]` impl, of one parameter, what React constructs it
  with: `constructor(props) { super(props); .. }`, `super` given its
  parameter first (`js::ExprKind::Super`), then its statements and
  fields as ADR 0761's constructor sets them.
- **A function of no `self` is its `static`**, `static greeting() { .. }`;
  a constant its prototype's, `Label.prototype.limit = 3;` after it, as
  0761's are.
- **It's where its impl is**: a class of no constructor in the module of
  its impl of the binding's class (`classes::home`), which imports that
  class, `import { Component } from "react"`, as a binding a body names
  is imported; one that writes its constructor implements the class in
  its constructor's module, or is refused.
- **It's exported as its struct is**, `pub`, though it writes no `pub`
  constructor or static, which ADR 0761 exports a class by: React
  constructs it, so a `pub` struct's class is a value JS is given, and
  declared `export declare class`, not its type alone.
- **A Rust call of its method is JS's**, `label.render()`.
- **Its declaration extends the binding's class**, `export declare class
  Counter extends Component<CounterProps, undefined> {
  componentDidMount(): void; .. }`, each method its impl writes, of its
  impl's types.
- **Refused**, each saying what to write: two impls of the class, which
  extends one; a field that isn't `undefined` until set, of a class React
  constructs; a constructor of other than one parameter; a method named
  as a field; the binding's method written; the impl in another module
  than its constructor's.

`render` gives `Element`, as a function component does: an impl that
writes `-> JSX::Element` for a trait's `-> impl Rendered` is warned of by
rustc's `refining_impl_trait`, which a port can't allow away.

### One way, beside ADR 0761's constructor function

This isn't a second form of a class. `#[rust_js::class]` is the one way
to make a JS class; what JS it is follows from what the Rust says it
extends, not from a choice the author makes: a class that extends a
binding's class is an ES class, as `extends` and `super(props)` can only
be written in one, and one that extends nothing is ADR 0761's constructor
function, which React's own ReactBaseClasses.js is, and which its
`ComponentDummy` is called as without `new`, which an ES class can't be.
The superclass is said by a trait impl, which rustc checks, its props
and methods typed, as Rust says what a type is: no attribute.

## Why

- **It's upstream's JS**: `class Surface extends React.Component {
  componentDidMount() { .. } render() { .. } }`, read line by line.
- **It's Rust a Rust React user writes**: a struct and an impl of the
  framework's component trait, as Yew's `impl Component for Model`,
  `self.props()` typed, the lifecycle methods' names and signatures
  checked by rustc.
- **React sees a real class**: `instanceof Component`,
  `Counter.prototype.isReactComponent`, `setState` and `forceUpdate` from
  its prototype, and a JS `class Loud extends Counter` whose `super.render()`
  is the Rust's.

## Alternatives

- **`#[rust_js::class(extends = "react#Component")]`**: an attribute
  rustc checks nothing of, `this.props` untyped, as ADR 0761 rejected.
- **A field of the base, `base: Component<P>`, `Deref` to it**: no JS
  instance has one; `self.base.props` would read through what isn't
  there.
- **Every class an ES class**: React's `ComponentDummy` and old callers
  call a class without `new`, which ADR 0761's form is for.
- **Associated types, `type Props; type State;`**: associated type
  defaults aren't stable, so each impl would write `type State = ();`;
  TypeScript's `Component<P = {}, S = {}>` is a class of type parameters
  with defaults, as the trait is.
- **The trait's lifecycle defaults the class's**: an empty
  `componentDidUpdate` React calls on every update, and a
  `shouldComponentUpdate` it would read as written.

## Consequences

- A class component is an element of `jsx!`, `<Counter label="a" />`
  (amended below, rd331i).
- `this.state` and `setState` aren't bound yet: `S` types
  `componentDidUpdate`'s `prevState`, `undefined` of `()`. Nor is
  `PureComponent`, a trait of its own.
- A call of the trait's method on a `C: Component<P>` of unknown `Self`
  is refused, as what may have no JS.
- The react crate imports by name, `import { Component } from "react"`,
  where ReactART.js writes `import * as React from 'react'` and
  `React.Component`.
- `react/coverage.txt` counts `Component` bound: a value a trait's
  `types` names is bound, as a class is a value.

## What holds it

- `test/class-components.test.ts`, run with real React in happy-dom: a
  class React constructs and one that writes its constructor, their JS
  and declarations, mount, update and unmount in order, `forceUpdate`,
  `instanceof Component`, `isReactComponent`, `setState`, a JS subclass's
  `super.render()`, a static, a Rust call of a method, a class alone in
  its module importing what it extends; and each refusal's message.
- Mutations `subclass-impl-of-no-class`, `subclass-impl-anywhere`,
  `subclass-binding-method-written`,
  `subclass-method-named-as-field-allowed`, `subclass-extends-nothing`
  (`lower/classes/subclasses.rs`); `subclass-methods-not-the-prototypes`,
  `subclass-unexported`, `subclass-homeless`, `subclass-of-two-allowed`,
  `subclass-without-new-refused`, `subclass-field-set-after-allowed`,
  `subclass-new-of-any-arity`, `subclass-super-unconstructed`,
  `subclass-declared-its-type-alone` (`lower/classes.rs`);
  `subclass-a-constructor-function`, `subclass-static-on-the-prototype`
  (`to_oxc/statements.rs`); `super-printed-as-this`
  (`to_oxc/expressions.rs`); `subclass-doc-unattached` (`to_oxc.rs`);
  `subclass-method-called-on-its-impl` (`lower/traits/calls.rs`);
  `subclass-declared-extending-nothing` (`lower/declarations/classes.rs`);
  `subclass-declared-of-the-traits-generics`,
  `subclass-declares-the-traits-defaults`
  (`lower/declarations/functions.rs`); `superclass-unimported`
  (`lower/analysis.rs`); `subclass-impl-refused`
  (`lower/analysis/validation.rs`).

## Amendment: each method is given `self: &'static Self` (rd331b)

Decided by the user (2026-11-01). A ref callback that sets a field of the
instance, ReactART's `ref={ref => (this._tagRef = ref)}`, was refused by
rustc: a callback React keeps is `'static`, and `&self` isn't. **Each of
`react::Component`'s methods takes `self: &'static Self`**, and `props`
gives `&'static P`: React makes and keeps the instance, as any JS object,
which rust-js has as `&'static` already, `&'static Element`. The receiver
says what's true; nothing extends a lifetime unchecked. So
`ref={move |r| self._tag_ref.set(r)}` compiles, and is `ref={(r) => {
this._tagRef = r; }}`: a block, as the closure gives `()`, where
upstream's `(this._tagRef = ref)` gives the element, which React reads
only for a cleanup function. An impl may still write `&self`, rustc's more
general signature, where it reads nothing of the trait's; `self.props()`
takes the `'static` one. Rejected: a `self.this()` binding, a second way to
reach the same instance; `Rc<Self>` receivers, which no JS object has.
Held by `test/class-components.test.ts`, whose `componentDidMount` sees
the paragraph its ref callback set.

## Amendment: a class that extends nothing keeps ADR 0761's form

Decided by the user (2026-11-01): react-art's LinearGradient,
RadialGradient and Pattern stay constructor functions, ADR 0262's second
case, listed in its Consequences and the guide's differences (37).

## Amendment: a class component is an element of `jsx!` (rd331i)

`<Surface width={..} />` of a class component was `react::component(Surface,
..)`, which rustc refused: a braced struct is no value. JS renders the
class, `createElement(Surface, ..)`. **`jsx!` renders a class component as
it renders a component function**: the expansion that writes a function's
companion, before names resolve (ADR 0213), writes one for a
`#[rust_js::class]` struct whose impl of `Component` is beside it, `impl
Component<SurfaceProps> for Surface`, read by the trait's name as a
function component is read by its signature (`parser::classes`). Its
companion, a macro of the struct's name and visibility, which a `use`
imports with the struct, builds the impl's props, by their own companion
where they have one, so an `Option` not written is left out, and gives
them to `react::component(react::class_component::<Surface, _, _>(), ..)`:
rustc checks the struct implements react's `Component` of those props, and
`class_component`'s JS is its class, as `js::class::<T>()`'s is, named by
its constructor or, of one React constructs, by a method of its impl
(`classes::member_naming`). The element is JSX's, `<Surface width={2} />`,
its props as written. Case N: React constructs the class it's given, as of
JS's.

One way: the element of a component function and of a class are the same
JSX, of one companion form; no `component::<Surface>(..)` call of a type.
Not yet: a `ref` of a class's element, the instance; and a class whose
impl of `Component` is in another module than its struct, which has no
element. Held by `test/class-components.test.ts`'s "a class component is
an element jsx! renders, its class given its props", rendered by React,
of one React constructs, one that writes its constructor and one of no
props, across modules, and the mutations `class-component-props-unread`,
`class-component-of-no-class` (`jsx_syntax/parser.rs`) and
`class-named-by-its-constructor-alone` (`lower/classes/subclasses.rs`).

## Amendment: `super(props)` comes before what the constructor's body writes first

`super(props)` had no place, so a comment Text's constructor starts with,
`// We allow reading these props..`, printed before it, where upstream
writes it after. `super(props)` is placed where the constructor is named,
before its body, so all its body writes, a comment too, follows it. Held
by `test/class-components.test.ts`'s "a constructor's comment comes after
super(props)" and the mutation `super-placed-nowhere` (`lower/classes.rs`).

## Amendment: a class component's `ref` is its instance

`<Gauge ref={..} />` of a class component was a field of its props,
`GaugeProps { r#ref: .. }`, which rustc refused: its props have none.
React gives a class's `ref` the instance it constructed, `None` as it
unmounts. **The companion's `@ref` arm gives the element its `ref` by
`react::class_ref::<Gauge, _>(element, reference)`**, `prop ref`, whose
`IntoRef<&'static Gauge, _>` rustc checks: a callback of `Option<&'static
Gauge>` or a `RefObject` of one. The JSX is JSX's, `<Gauge label="g"
ref={(gauge) => { .. }} />`. Case N: React does what it does of a JS
class's `ref`.

A class of no fields, `pub struct Gauge {}`, was nullish by its shape, as
a unit struct is, so a `Some` of its instance was boxed and read back,
`$someValue(gauge)`: its value is the object its constructor made, never
`undefined`, so it's no unit struct's (`nullish_by_its_shape`). Case N.

Held by `test/class-components.test.ts`'s "a class component's ref is its
instance", rendered by React DOM, and the mutations
`class-component-ref-a-prop` (`jsx_syntax/parser.rs`) and
`class-of-no-fields-nullish` (`lower/representation/options.rs`).

Not yet, still: an element of a class whose impl of `Component` is in
another module than its struct. The companion, a macro beside the struct
that `use` imports with it, is written before names resolve, of the
impl's props as its module names them; another module's impl names the
struct through its own `use`s, which the expansion would have to resolve
as rustc does to pair them.

## Amendment: an optional `OnceCell` is a field set after (rd331l)

ReactART's Surface sets `this._mountNode = createContainer(..)` once, as it
mounts, and reads it in each later method while the reconciler runs. A
`RefCell<Option<Rc<FiberRoot>>>` borrowed across that call is refused
(ADR 0863), as the reconciler may borrow it; the Rust for a value set
once is a `OnceCell`, which a class React constructs was refused. **An
optional `OnceCell`, `#[rust_js::optional]`, is a field set after**: absent
until it's set (ADR 0280, amended), `undefined` as a field React's
instance gains after is. Its `set` is Rust's, set where it's still empty,
`if (this._root === undefined) { this._root = .. }`, as rustc can't know
React mounts the instance once; its `get().unwrap()` is `$unwrap`. Held by
`test/class-components.test.ts`'s "a class React constructs holds an
optional OnceCell, set once" and the mutation `class-once-cell-refused`
(`lower/classes/subclasses.rs`).
