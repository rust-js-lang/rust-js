# 0597. Whether the host has a global is `js::is_defined`'s

Status: Accepted. Supersedes [0583](0583-a-global-some-hosts-lack-is-an-option.md).
Amended by [0645](0645-is-defined-of-a-static-is-of-the-static.md): a
static is given as itself.
Extends [0021](0021-js-interop.md)'s globals.

Case ([0262](0262-when-rust-and-js-disagree.md)): A. A global the host
has is read as it was; asked of one it hasn't, the answer is `false`, where
reading it threw.

## Context

ADR 0583 made a global some hosts lack an `Option`: `js::set_immediate`,
`None` in a browser. React's scheduler asks for three:

```js
const localSetTimeout = typeof setTimeout === 'function' ? setTimeout : null;
..
if (typeof localSetImmediate === 'function') {
  ..
} else if (typeof MessageChannel !== 'undefined') {
  ..
} else {
  ..
}
```

and its tests delete `setImmediate`, `MessageChannel` and `setTimeout`
before they load it, as jsdom and SSR hosts lack them. Each global had one
declaration (ADR 0583), so `MessageChannel` and `setTimeout`, bound as
always there, couldn't be asked of. Made `Option`s by the same rule, so
would most of webapi: jsdom, which React apps' tests run in, lacks 176 of
the 225 classes webapi constructs, `Request`, `TextEncoder` and
`ReadableStream` among them. Every host lacks something, so "an `Option`
where some host lacks it" either spreads `Option` through every binding or
is decided case by case, a list nobody could guess.

The user's decision: be consistent, no surprise, consistent and intuitive.

## Decision

**A global is declared as TypeScript declares it**, its own type, read as
its name: `js::set_immediate` is a function, as `webapi::message_channel::new`
and `js::set_timeout` are.

**Whether the host has it is one question, `js::is_defined(binding)`**, of
any global's binding, written as JS writes it:

```rust
if js::is_defined(js::set_immediate) {
    js::set_immediate(task);
} else if js::is_defined(webapi::message_channel::new) {
    ..
}
```

```js
if (typeof setImmediate !== "undefined") {
  setImmediate(task);
} else if (typeof MessageChannel !== "undefined") {
```

The binding is named by the argument's type, a function item's, so a
temporary between, `let make = absent; js::is_defined(make)`, asks of the
same global: unlike Scala.js's `js.typeOf`, which writes `typeof x` only
where `x` is written as the argument. What isn't a global's own binding is
refused: a closure, the crate's own function, a method, an import, or a
path from a global, `console.log`, whose `console` may be what's missing.

An `Option` static is now what its type says of the value, `undefined` or
`null` where it is, as TypeScript's `declare var x: T | undefined`.

## Why

- **One rule, the same for every global**: it's what TypeScript and the
  host's docs say, never a list of which hosts lack what.
- **It's the JS people write**, `typeof X !== "undefined"`, where they
  write it: a program asks where it cares, as React's scheduler does, and
  everywhere else calls the binding as TypeScript lets it.
- **No surprise**: `js::set_timeout(..)` is called as JS calls it, not
  unwrapped in every app for the hosts that lack it.

## Alternatives

- **Keep ADR 0583, more `Option`s**: `MessageChannel`, `setTimeout`, and by
  the same rule most of webapi. Rejected: the rule can't be applied, or
  makes every binding an `Option` no TypeScript has.
- **A constructor that gives an `Option`**: `message_channel::new()` `None`
  where the class is missing. Rejected: the same list of which classes.
- **`js::is_defined("MessageChannel")`**: a name as text, unchecked.
  Rejected.

## Consequences

- What Rust checks is the call, not the host: a program that calls a
  binding the host lacks throws a `ReferenceError` there, as the JS does.
  `js::is_defined` is how a program that runs where it may be missing asks.
- A path's every part is asked, `typeof console !== "undefined" && typeof
  console.log !== "undefined"`, as a host may lack any; and a method of a
  type one global holds, `js::is_defined(webapi::Performance::now)`, is
  that global's, `typeof performance !== "undefined" && typeof
  performance.now !== "undefined"`, React's `getCurrentTime` asks; a type
  two globals hold is refused, saying neither.
- A global's value is asked of by its static, `js::is_defined(&webapi::performance)`:
  the static the argument refers to, followed through MIR's temporaries,
  `typeof performance !== "undefined"`. A variable's, or the crate's own
  static's, is refused. (Amended by ADR 0645: the static is given as
  itself, `js::is_defined(webapi::performance)`, a reference to it
  refused.)

## What holds it

- `test/lowering.test.ts`, "js::is_defined is whether the host has a
  binding's global" and "js::is_defined of what isn't a global's binding
  is refused".
- The guide's [global scope](../guide/interop/global-scope.md#a-global-some-hosts-lack).
- Mutations: the `typeof` unwritten, of any path, of the crate's own
  function.
