# 0673. A function is a JS value of any shape, whose property `js::set` writes

Status: Accepted. Amends [0225](0225-unknown-values.md), whose
`js::unknown` took a `Defined` value only, and extends
[0267](0267-on-load.md), whose module's statements run when it's loaded.
Extended by [0965](0965-a-shared-function-is-of-any-shape.md): an `Rc`
of a function is of any shape too, the function itself.

Case: N ([0262](0262-when-rust-and-js-disagree.md)): a function is JS's
own object, and its property is written where the program writes it.

## Context

React's `shared/ConsolePatchingDev.js` marks the function it puts in
`console`'s place, once, when the module loads:

```js
function disabledLog() {}
disabledLog.__reactDisabledLog = true;
```

React DevTools reads `__reactDisabledLog` to tell React's stand-in from
the console's own `log`. A module's statements are `js::on_load! { .. }`
(ADR 0267), and a property of a value of any shape is `js::set`'s (ADR
0225), so the port wrote:

```rust
fn disabled_log() {}
js::on_load! {
    js::set(js::unknown(&disabled_log), "__reactDisabledLog", true);
}
```

and rustc refused it: "the trait `js::Defined` is not implemented for fn
item `fn() {disabled_log}`". `js::unknown` took a `Defined` value, what's
never `undefined` nor `null`, and no function was one, though each is a
JS function: an object, never nullish, as a struct is. The port left the
mark out, so DevTools couldn't tell React's `log` from the console's.

The printer set a function apart from what follows it by a blank line,
so the mark, once written, was a blank line below its function, where
upstream has it right after.

## Decision

**A function is a JS value of any shape: `js::unknown(&f)` is `f`, a
function item, a closure, a `fn` pointer or a `dyn Fn`, whose property
`js::set` writes, and `js::get` reads, as an object's.** The port writes
what it tried:

```rust
fn disabled_log() {}
js::on_load! {
    js::set(js::unknown(&disabled_log), "__reactDisabledLog", true);
}
```

```js
function disabledLog() {}
disabledLog.__reactDisabledLog = true;
```

- **`js::unknown` takes an `AnyShape`**, a `Defined` value or a
  function of any arity up to 12, `FnOnce` of any arguments; `js::unknown_of`
  too, a closure given by value. `AnyShape<M>`'s `M` says which, a marker
  rustc infers, `()` of a `Defined` value and the `fn` pointer of a
  function's signature, which no program names. Its JS is the value
  itself, as it was.
- **A function and what sets a property of it are one statement block**:
  no blank line between `function f() {}` and `f.x = ..;`, as a person
  writes them, as `let x;` and what sets it are (ADR 0601).
- **Reading one back** is `js::get`'s, of the function as it's held:
  `let log: ConsoleFn = console::log;
  js::get(js::unknown(&log), "__reactDisabledLog")` is `const log =
  console.log; log.__reactDisabledLog`. A variadic binding as a value is
  the `fn` of its rest arguments (ADR 0667).

## Why

- **One way to write a property, of every object**: `js::set(js::unknown(&x),
  key, value)` is a struct's, an element's and now a function's.
- **The Rust a person writes**: `&disabled_log`, no cast and no turbofish,
  where stable Rust coerces a function item to nothing a generic
  parameter asks for.
- **A function isn't `Defined` only because Rust can't say it**: a
  function item's type has no name, and `impl<F: Fn()> Defined for F`
  would overlap `&T`'s, `Box<T>`'s, and each arity's other. A marker per
  arity is a separate impl of a separate trait, so none overlaps, and
  rustc picks the one that holds.

## Alternatives

- **`js::unknown(&(disabled_log as fn()))`, a `fn` pointer `Defined`**:
  stable Rust's way without a marker, a cast in every port, of a function
  that's already one.
- **A `#[rust_js::property]` attribute of the function**, a typed form of
  TypeScript's expando property, `disabledLog: { (): void;
  __reactDisabledLog: boolean }`: a second way to set a property, beside
  `js::set`, for one kind of object.
- **`Defined` for each `dyn Fn` signature**: `js::unknown::<dyn Fn()>(&f)`,
  a turbofish, as rustc coerces only where the type is known.

## Consequences

- `truthy` still takes a `Defined` value: a function is always truthy,
  as a closure is (ADR 0298), so there's nothing to ask of one.
- A program that names `js::unknown`'s type parameter, `js::unknown::<T>`,
  names its marker too, `js::unknown::<T, _>`; none does.

## What holds it

- `test/js-object.test.ts`, "a function's property is set where its
  module loads it": `disabledLog.__reactDisabledLog = true;` right after
  its declaration, read back from Rust by `js::get` and by DevTools' read,
  `console.log.__reactDisabledLog`, run in Node; a closure's, a `fn`
  pointer's and a `dyn Fn`'s properties set, and `Object.keys` of the
  closure. "console is a value, its functions read as values": React's
  `ConsolePatchingDev` as the port writes it, marked.
- Mutation `function-mark-apart`: the mark a blank line below its
  function. Later (2026-10-28) gone, with the formatter's rule it mutated:
  since ADR 0865 a blank line is where the Rust has one, so a mark the
  program writes right after its function is right after it in the JS by
  that rule, and the formatter's own spacing only meets statements the
  Rust tells nothing of, where an `on_load!` body's mark comes before its
  function, a macro's too.
