# 0695. `new` of a function value is JS's `new`, `js::new`

Status: Accepted. Extends [0225](0225-unknown-values.md), whose
operators of a value of any shape were `js::get`, `js::set` and
`js::has`.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: it's JS's `new`,
which runs the constructor JS runs and throws where JS throws.

## Context

React's `shared/ReactComponentStackFrame.js` constructs a class
component without `Reflect.construct`:

```js
// eslint-disable-next-line no-new
new fn();
```

The js crate had `reflect::construct(target, &[], None)`, which is
`Reflect.construct(fn, [])`, a different call: nothing wrote `new fn()`.
TypeScript refuses `new` of a `Function`, which has no construct
signature; it allows it of an `any`, giving `any`.

## Decision

**`new value(...arguments)` is `js::new(value, arguments)`, of a value
of any shape, as `js::get` reads one: its arguments a rest parameter's,
a tuple, `()` of none, or a slice spread (ADR 0640).**

```rust
js::new(js::unknown(r#fn), ());
js::new(class, (a, "b"));
js::new(class, rest);
```
```js
new fn();
new class$(a, "b");
new class$(...rest);
```

- **What it makes is a `&'static Unknown`**, an object, as
  `Reflect.construct`'s is.
- **What's no constructor JS throws a `TypeError` of, a panic**: a
  closure is an arrow function, so `new` of one throws; a constructor
  Rust writes is a `fn` item (ADR 0696).
- **A binding's form `new this()`**: `new this(..)`, `this` its first
  parameter, as `this()` calls it (ADR 0024's forms).

## Why

- **JS's operator, of what TypeScript allows it**: `any`, as `js::get`,
  `js::set` and `js::has` are of a value of any shape. One family: each
  of JS's operators on any value is a `js::` function of an `&Unknown`.
- **`Reflect.construct` is another function**: a binding of `Reflect`'s,
  with its own `newTarget`, as `Reflect.get` is beside `js::get`.

## Alternatives

- **`js::new` of a `&dyn Fn() -> Option<&Unknown>`**, TypeScript's
  `Function` (ADR 0694): TypeScript refuses `new` of one, and a function
  of other arguments, `dyn Fn(&Props)`, isn't one.
- **A slice of arguments, `&[Option<&Unknown>]`**, as `Reflect.construct`
  takes: `new fn(...[])`, an array where JS writes none.

## What holds it

- `test/js-object.test.ts`, "new of a function value is JS's new": of
  none, two and a spread slice, run on a class; and of a closure, which
  throws, caught.
- `test/js-object.test.ts`, "React's component stack frame is found as
  upstream finds it": `new fn();`.
- Mutation `js-new-this-called`: `new this()` a call, `fn()`.
