# A property of a function is set as an object's is

Status: Draft. Would extend [0267](../0267-on-load.md), whose module's
statements run when it's loaded, and [0225](../0225-unknown-values.md),
whose `js::set` writes a property of a JS value of any shape.

Case: N ([0262](../0262-when-rust-and-js-disagree.md)): the function is
JS's own object, and the property is written where the program writes it.

## Context

React's `shared/ConsolePatchingDev.js` marks the function it puts in
`console`'s place, once, when the module loads:

```js
function disabledLog() {}
disabledLog.__reactDisabledLog = true;
```

React DevTools reads `__reactDisabledLog` to tell React's stand-in from
the console's own `log`. A module's statements are `js::on_load! { .. }`
(ADR 0267), so the port writes:

```rust
fn disabled_log() {}

js::on_load! {
    js::set(js::unknown(&disabled_log), "__reactDisabledLog", true);
}
```

and rustc refuses it: `js::set` writes a property of an `&Unknown`, and
`js::unknown` takes a `Defined` value, which no function is: "the trait
`js::Defined` is not implemented for fn item `fn() {disabled_log}`". A
function item, a `fn` pointer, a closure and a `dyn Fn` are each a JS
function, never `undefined` nor `null`, so each is a JS value of any
shape, as a struct is; nothing says so.

## Proposal

**A function is `Defined`, so `js::unknown(&f)` is the function as a JS
value of any shape, whose property `js::set` writes: `disabledLog.__reactDisabledLog
= true;`.**

- **A `fn` pointer and a `dyn Fn` are `Defined`** in the js crate,
  `unsafe impl<R> Defined for fn() -> R {}` and its arities, and `unsafe
  impl Defined for dyn Fn..`. A function item has no type a crate can
  name: `js::unknown` would take it through a trait the compiler
  implements for it, or rust-js would read `js::unknown(&f)` of a
  function item as the function, its MIR's `ReifyFnPointer` before
  `unknown`'s call, as ADR 0602 reads a binding's.
- **Its JS is the function**: `js::unknown(&disabled_log)` is
  `disabledLog`, `js::set` of it `disabledLog.__reactDisabledLog = true`,
  where `js::on_load!` writes it, after the function's declaration.
- **Its type checks it**: `js::get(js::unknown(&f), "name")` reads the
  function's `name` as an `Unknown`'s, which TypeScript would type as
  `Function`'s `name: string`.

## Open gates

- Whether a function item takes the trait route or the reified one, and
  which reads more as Rust: `js::unknown(&(disabled_log as fn()))` is
  what stable Rust allows without either.
- Whether TypeScript's expando property of a function declaration,
  `disabledLog.__reactDisabledLog = true` typing `disabledLog` as `{ ():
  void; __reactDisabledLog: boolean }`, wants a typed form instead: a
  `#[rust_js::property]` of the function, say.

## What would hold it

- A test in `test/js-object.test.ts`: React's `disabledLog` marked when
  its module loads, `disabledLog.__reactDisabledLog = true;` after its
  declaration, read back from JS.
- A mutation that leaves the function out of `Defined`, or the reified
  pointer unread.
