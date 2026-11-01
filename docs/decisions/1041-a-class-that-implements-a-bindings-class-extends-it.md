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
    fn component_did_mount(&self) {
        self._mounts.set(Some(self._mounts.get().unwrap_or(0.0) + 1.0));
        println!("mount {}", self.props().label);
    }

    fn component_did_update(&self, prev_props: &CounterProps, _prev_state: &()) {
        println!("update {} to {}", prev_props.label, self.props().label);
    }

    fn render(&self) -> JSX::Element {
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

- A class component isn't yet an element of `jsx!`, `<Counter label="a"
  />`: a struct is no value in Rust, which `jsx!`'s `component(Counter,
  ..)` passes; JS renders it, `createElement(Counter, ..)`. Its own gap.
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
