# 0663. `console` is a value, and a `fn` of rest arguments is the JS function

Status: Accepted. Amends [0602](0602-a-binding-taken-as-a-fn-is-its-function.md),
whose variadic binding taken as a `fn` stayed an arrow that read the
global at each call, and extends [0640](0640-a-rest-parameter-is-a-tuple.md),
whose rest arguments were a tuple, a slice or a `Vec`.

Case ([0262](0262-when-rust-and-js-disagree.md)): C, results would change:
a `fn` is fixed where it's taken, so it's the function read there, and a
Rust function or a `dyn Fn` its call wouldn't spread to is refused.

## Context

React's `shared/ConsolePatchingDev.js` saves `console`'s methods, replaces
them, and puts them back:

```js
prevLog = console.log;
// ..
Object.defineProperties(console, { log: props, /* .. */ });
// ..
Object.defineProperties(console, { log: {...props, value: prevLog}, /* .. */ });
```

TypeScript has `declare var console: Console`; WebIDL a `namespace
console`, which webapi binds as a module of functions, `console::log((..))`
(ADRs 0313, 0640). So there was no `console` to give
`Object.defineProperties`. And a method read as a value had one form, a
binding taken as a `fn` (ADR 0602), which a variadic's wasn't:
`fn(&'static [Option<&'static Unknown>])` of `console::log` was `(data)
=> console.log(...data)`, which reads `console.log` at each call. Restored
as `console.log`, that arrow calls itself.

## Decision

**`console` is a value, `webapi::console`, a `&'static Console` beside
the module of its functions; and a variadic binding taken as a `fn` of
rest arguments as a value, `&'static dyn js::Rest<T>`, is its JS function
itself, which a call through the `fn` spreads its arguments to.**

```rust
type ConsoleFn = fn(&'static dyn Rest<dyn IntoUnknown>);

PREV_LOG.set(Some(console::log));
object::define_properties(webapi::console, &Logs { log: props, .. });
let log: ConsoleFn = console::log;
log(&("named", 1));
```

```js
prevLog = console.log;
Object.defineProperties(console, { log: props, .. });
const log = console.log;
log("named", 1);
```

- **A namespace that's a global value too is a static of a type of no
  members of its own**: webapi's generator gives each of its namespaces
  that TypeScript declares a `var`, `console` alone, a `Console` type and
  a `console` static at the crate root, as `document` is. Its functions
  stay its module's, `console::log`, Rust telling the value from the
  module as it tells a value from a type.
- **A method read as a value is a binding taken as a `fn`**, the one form
  (ADR 0602), not a getter of `Console`'s: a getter would be a second way
  to read a binding, and its `dyn Fn` couldn't spread a rest.
- **Rest arguments as a value are `&dyn js::Rest<T>`**: `js::Rest` is
  implemented for a reference to one, so a variadic binding takes it,
  spread, `console.warn(...data)`, and a `fn` of one is a variadic
  binding's instance, whose arrow `(...data) => console.log(...data)`
  passes its parameters on, so it's the function itself. A call of a `fn`
  pointer whose last parameter is one gives it as a variadic binding's
  call gives its rest: a tuple written out as its items, `log("named",
  1)`, any other spread. One place, `given_as_rest`, says how for both.
- **What a call wouldn't spread to is refused**: a Rust function or
  closure taking `&dyn js::Rest<T>`, which a `fn` pointer of its type
  would be given only the first item, and a `dyn Fn` of one, whose call
  gives them as one array. "rust-js does not support a `&dyn
  js::Rest<T>` parameter, rest arguments as a value, of a Rust function
  or closure yet"; a Rust function takes a slice.
- **`groupEnd(): void` is a `fn()`**: a binding's `fn` is of its own
  signature, TypeScript's.

## Why

- **`console` is TypeScript's value**: the program writes it where JS
  does, `Object.defineProperties(console, ..)`.
- **A `fn` is fixed where it's taken**: the function read there, as ADR
  0602 made every other binding's; React's restore puts back the
  function it saved.
- **The type says the call**: a `fn` of a `&dyn Rest` is called with its
  arguments spread, as JS calls `console.log`; a `fn` of a slice is
  given the slice, as a Rust function of one is.

## Alternatives

- **A `Console` type with getters, `console.log()` a `&'static dyn
  Fn(..)`** (ADR 0653): a second way to read a binding as a value, and a
  `dyn Fn` has no rest to spread.
- **The arrow of a slice, capturing the function where it's taken**,
  `((log) => (data) => log(...data))(console.log)`: right, and no JS a
  person writes.
- **A `fn` of a tuple**: one arity per type, and called with an array
  all the same.

## Consequences

- **A `fn` of a generic's instance is of one lifetime**, so of
  `&'static dyn Rest`: `log(&("named", 1))` of literals, which Rust
  promotes; rest arguments that borrow a local are given to the binding
  itself, `console::log((name,))`.
- A Rust function can't take rest arguments as a value; it takes a slice,
  `&[&dyn IntoUnknown]`, and gives it spread, `console::log(values)`.

## What holds it

- `test/js-object.test.ts`, "console is a value, its functions read as
  values": React's `disableLogs` and `reenableLogs`, each method saved,
  replaced and put back, the same function; a `fn` of rest arguments
  called spread. "rest arguments as a value are refused where a call
  wouldn't spread them": a Rust function's and a `dyn Fn`'s.
- `test/webapi-coverage.test.ts`: the crate is its generator's, `console`
  among its globals.
- Mutations: `rest-value-unrecognized`, `rest-value-refused-by-a-call`,
  `rest-value-arrow-unspread`, `rest-value-passed-on-unseen`,
  `rest-value-pointer-call-unspread`, `rest-value-parameter-allowed`,
  `rest-value-dyn-fn-allowed`.
