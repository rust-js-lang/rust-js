# 0964. `js::SymbolFor` itself is `Symbol.for`, a function a host may lack

Status: Accepted. Extends [0634](0634-a-registered-symbol-is-a-const.md),
`js::SymbolFor(key)` the registry's symbol, to its constructor as a value,
and [0942](0942-whether-a-global-is-a-function-is-is-function.md),
`js::is_function` of a global's binding, to it.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. `Symbol.for` reads no
`this` and only its first argument, so the function and an arrow calling it
are one; a `typeof` reads and runs nothing, so one an earlier test of an
`&&` says changes nothing gone.

## Context

React's test selectors make their selector types symbols where the host
has them:

```js
if (typeof Symbol === 'function' && Symbol.for) {
  const symbolFor = Symbol.for;
  COMPONENT_TYPE = symbolFor('selector.component');
  ...
}
```

`js::is_function(js::SymbolFor)` was refused, of no global's binding, and
`let symbol_for = SymbolFor;` was `const symbolFor = (value) =>
Symbol.for(value);`, read by no call: `symbol_for(key)` was
`Symbol.for(key)`.

## Decision

- **`js::SymbolFor`'s constructor as a value is `Symbol.for`**
  (`constructor_value`, `representation::symbol_for_function`), and a call
  of it a variable holds is a call of that variable (`called_as_js`), as a
  binding's is: `const symbolFor = Symbol.for; symbolFor(key)`.
- **`js::is_function(SymbolFor)` and `js::is_defined(SymbolFor)` ask of
  `Symbol.for`**, as of a binding's global: `typeof Symbol !== "undefined"
  && typeof Symbol.for === "function"`.
- **In an `&&`, a `typeof p !== "undefined"` an earlier `typeof p ===
  "function"` or `!== "undefined"` says is gone** (`prepare/typeofs.rs`):
  `js::is_function(js::symbol::new) && js::is_function(SymbolFor)` is
  `typeof Symbol === "function" && typeof Symbol.for === "function"`.

Upstream's last test is `Symbol.for`'s truth; rust-js asks whether it's a
function, which is true exactly where it's there.

## What holds it

- `test/lowering-is-function.test.ts`, "js::SymbolFor is Symbol.for, asked
  whether it's a function and called where it's held", run.
- Mutations: `symbol-for-value-an-arrow` (`aggregates.ts`),
  `symbol-for-held-not-called` (`mir/dispatch.ts`), `symbol-for-not-a-global`
  (`mir/calls.ts`), `typeof-said-asked-again`, `typeof-other-read-said`
  (`prepare/typeofs.ts`).
