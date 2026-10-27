# 0972. A function that reads its `this` is `js::function::with_this`'s

Status: Accepted (the user's decision, 2026-10-27, th1). Amends
[0761](0761-a-rust-js-class-is-a-constructor-function-and-its-prototype.md)'s
`self` as `this`, which a function made in a method now reads through a
`const`.

Case ([0262](0262-when-rust-and-js-disagree.md)): none for the function:
its `this` is a parameter Rust sees, so what it reads is what JS gives it.
None for its name: Rust gives a closure no name, so the name a function
stored as a property has is the representation's, as a `let`'s arrow
takes the `let`'s by JS's own inference.

## Context

React's `initializeClassErrorUpdate` gives an error boundary a callback
that captures what it needs and reads the instance it's called on:

```js
update.callback = function callback() {
  logCaughtError(root, fiber, errorInfo);
  markLegacyErrorBoundaryAsFailed(this);
  this.componentDidCatch(error, { componentStack: stack !== null ? stack : '' });
};
```

`commitCallbacks` calls it as `callCallback(callback, context)`,
`callback.call(context)`. rust-js had no Rust for it: a closure is an
arrow ([0882](0882-a-field-of-statements-is-a-method.md)),
which never reads `this`; an object's methods
([0943](0943-an-object-of-methods-reading-this.md)) read `this` but capture nothing.
The nearest port read `fiber.stateNode` when the error was thrown, a
different instance wherever `commitCallbacks`' context differs, so
ReactFiberThrow was held.

## Decision

**`js::function::with_this(move |this, (a, b)| { .. })` is `function (a,
b) { .. }`**, its `this` JS's `this`:

- It's a binding, `link_name = "function"`, of a closure `F: Fn(Option<&'static
  Unknown>, A) -> R`, `A` a tuple of up to 12, giving a `Box<dyn Fn(..A) ->
  R + 'l>` (`js::function::Parameters`), which `js::unknown_of`,
  `function::call` and `function::apply` take, and Rust calls as any
  function, its `this` then `None`. An ordinary closure: rustc checks its
  captures, moves and borrows.
- Lowering (`binding_calls.rs`, `function_with_this`) makes the closure's
  arrow, `(this$, [a, b]) => ..`, a function of the tuple's items, `function
  (a, b)`, one it leaves out a parameter of its own, `(_, b)`; a tuple it
  reads whole is the array of them, `const pair = [pair$1, pair$2];`. Each
  read of its `this` is JS's, `js::read_as_this`, through each arrow in it,
  which sees its function's `this`; a function nested in it, another
  `with_this`'s, has its own, so it reads the outer one through `const
  this$ = this;`. A `this` the closure sets, `mut this`, is a `let` of its
  own, `let this$ = this;`, as JS's `this` can't be assigned.
- The closure is written in the call: one a variable holds is refused,
  naming the form.
- A class's method's `self` (0761) is read the same way: a function made
  in it reads it through `const counter = this;`, as it would read a
  handle's place (#359). A constructor whose instance such a function
  reads is refused.
- Preparation names it: stored as a property, `update.callback =
  function callback() { .. }`, where the property is a name a function
  may have, no word JS keeps, and one its body reads nothing by, which
  its own name would then be; a `let` of it is declared, `function inner()
  { .. }`, as an arrow of statements is (0882). Its `this` stays its own
  either way.

So the port writes:

```rust
update.callback.set(Some(js::unknown_of(function::with_this(move |this, ()| {
    let this: &'static ClassInstance = unsafe { js::cast(this) };
    ..
}))));
```

## Alternatives

- **An attribute on a `let` of the closure, `#[rust_js::this]`**: a
  closure's parameter would mean what its type doesn't say, and the
  closure couldn't be written in its call, as an attribute on an
  expression needs `stmt_expr_attributes` (ADR 0110).
- **A struct of the captures with a `#[rust_js::function]` impl**: not the
  Rust anyone writes for a callback, and a second capture analysis.
- **A wrapper of a closure a variable holds, `function (...a) { return
  f(this, a); }`**: a second form of the same function; the closure is
  written in the call instead.
- **Named by nothing, `update.callback = function () { .. }`**: correct,
  but not the name upstream writes, which devtools and stacks show.

## Consequences

- ReactFiberThrow's `initializeClassErrorUpdate` is upstream's `function
  callback()`, its `this` the instance `commitCallbacks` calls it on.
- `js::read_as_this` is the one rule for where a variable is JS's `this`,
  which `with_this` and a class's method and constructor ask.
- Preparation's arrow-to-method and arrow-to-declaration rewrites ask
  `js::reads_this`, which a function made in an arrow doesn't count for.
- Tested in `test/js-functions.test.ts`: `call`, `apply`, a method's and a
  bare call's `this`, nested functions, a class's method, a `this` set and
  a tuple read whole.
