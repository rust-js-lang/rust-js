# 0943. A `#[rust_js::object]` is an object literal of its methods, each reading `this`

Status: Accepted, the user's decision of 2026-10-26 that a plain object
literal with methods reading `this` has a form; its design chosen here.
Extends [0761](0761-a-rust-js-class-is-a-constructor-function-and-its-prototype.md)
(a `#[rust_js::class]`), whose prototype's methods these are the
object's own of, and [0047](0047-methods.md), whose object of functions
stays every other struct's impl.

Case ([0262](0262-when-rust-and-js-disagree.md)): A for what's kept, a
value's fields and its methods' meaning, `self` being what the method is
called on as JS's `this` is; and a compile error where Rust's can't be
kept: a method of `self` by value, which JS's `this` can't be, a method
named as a field, two values of one key, a struct of type parameters,
whose methods are one function for every value.

## Context

React's ReactFiberScope makes a scope's instance as an object literal of
its module's functions, each reading `this`, the instance it's called
on:

```js
function containsNode(this: $FlowFixMe, node: Object): boolean {
  let fiber = getInstanceFromNode(node);
  while (fiber !== null) {
    if (fiber.tag === ScopeComponent && fiber.stateNode === this) {
      return true;
    }
    fiber = fiber.return;
  }
  return false;
}

export function createScopeInstance(): ReactScopeInstance {
  return {
    DO_NOT_USE_queryAllNodes,
    DO_NOT_USE_queryFirstNode,
    containsNode,
    getChildContextValues,
  };
}
```

rust-js had no way to write it. A struct's impl is an object of functions
given the value (ADR 0047), `ScopeInstance.containsNode(scope, node)`,
which React's users, calling `scope.containsNode(node)`, can't call. A
`#[rust_js::class]` (ADR 0761) makes a constructor and its prototype,
`new ReactScopeInstance()`, whose methods aren't the object's own. A
field of a closure is a method of the literal (ADR 0882), but a Rust
closure never reads `this`.

## Decision

The user's decision: such an object has a form. Its design: **a struct
and its inherent impl, each marked `#[cfg_attr(rust_js,
rust_js::object)]`, are an object literal of its fields and its impl's
methods** (`classes.rs`, `Member::Own`):

```rust
#[cfg_attr(rust_js, rust_js::object)]
pub struct ScopeInstance {}

#[cfg_attr(rust_js, rust_js::object)]
impl ScopeInstance {
    pub fn contains_node(&self, node: &Unknown) -> bool { .. js::unknown(self) .. }
}

pub fn create_scope_instance() -> ScopeInstance {
    ScopeInstance {}
}
```

```js
function containsNode(node) { .. this .. }

export function createScopeInstance() {
  return { containsNode };
}
```

- **A method of `&self` or `&mut self` is a function of its impl's
  module**, `function containsNode(node) { .. }`: a `function`, so `this`
  is what it's called on, and its `self` is `this`, as a class's method's
  is (`receiver_as_this`). It's exported only where another module's
  code names it.
- **Each literal of the struct holds them**, after its fields, each its
  own property by the method's JS name, camelCase or its
  `#[rust_js::name]`, `DO_NOT_USE_queryAllNodes`: `{ queries: 0,
  containsNode }`, imported where the literal is another module's.
- **A Rust call of one is a call of the object's property**,
  `scope.containsNode(node)`, so `this` is the object, as JS's users call
  it; as a value it's the arrow of that call, `(a, b) =>
  a.containsNode(b)`, as a class's method's is.
- **Its `.d.ts` is an interface of its fields and its `pub` methods**,
  `containsNode(node: unknown): boolean`, as TypeScript's
  `ReactScopeInstance` is.
- **The impl's other items are any impl's** (ADR 0047): a function of
  no `self`, a constant, `ScopeInstance.make()`. A trait's impl is any
  type's: its methods a dictionary's, not the object's.
- **Refused, saying what to write**: a struct of no named fields; one of
  type or const parameters; one marked `#[rust_js::class]` too; an
  impl marked without its struct, or a struct without its impl; a method
  of `self` by value; a method named as a field.

## Why

- **It's the class's form, its methods where the object has them**: one
  example of either reads as the other, `&self` `this` in both, a call
  `scope.containsNode(..)` in both; only where the methods live differs,
  the prototype or the object, which is what the attribute says.
- **It's idiomatic Rust**: a type and its methods, `scope.contains_node(
  node)`, nothing a port writes for rust-js but the attribute.
- **Its JS is upstream's**, the module's functions and the literal of
  them, shorthand where a function's name is its property's.

## Alternatives

- **Functions whose first parameter is named `this`, as a binding's is,
  and a struct of `fn` fields holding them**, `ScopeInstance {
  contains_node }`: its JS is upstream's too, but its Rust isn't a
  type's methods: a Rust caller writes `(scope.contains_node)(&scope,
  node)`, and a call whose first argument isn't the object the field is
  read from has no JS method call. Equally upstream's JS, further from
  Rust's and from the class's form.
- **A `#[rust_js::class]` made by a struct literal outside `new`**, its
  methods the literal's own: one attribute of two representations, an
  instance of the prototype or an object holding them, by where it's
  made.
- **Each method written in the literal, `{ containsNode(node) { .. }
  }`**: every literal a copy of each function, where upstream's has one
  of each; and not upstream's JS.

## What holds it

- `test/classes.test.ts`, "an object's methods are its literal's own,
  functions of its module reading this" (run as JS, two instances each
  its own `this`, and its `.d.ts`), "an object made in another module
  holds its impl's module's functions" and "what an object is refused
  says what to write".
- Mutations `object-self-a-parameter`, `object-method-a-prototypes`,
  `object-by-value-unchecked`, `object-unmarked-impl-unchecked`,
  `object-generic-allowed` (`classes.rs`), `object-method-value-itself`
  (`lower.rs`), `object-literal-without-methods` (`aggregates.rs`),
  `object-method-called-as-a-function` (`mir/calls.rs`),
  `object-methods-undeclared` (`declarations.rs`) and
  `object-method-its-types` (`analysis/naming.rs`).
