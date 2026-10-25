# 0891. A window's constructor is the window's: `window.new_error_event(..)`

Status: Accepted. Extends [0024](0024-web-crate.md)'s `link_name`
forms, `new X` of a method; [0281](0281-webapi-covers-typescripts-dom.md)'s
webapi, each class's constructor a `Window` method; and
[0597](0597-whether-the-host-has-a-global-is-is-defined.md)'s and
[0624](0624-is-defined-of-a-path-of-bindings.md)'s `js::is_defined`, of
a constructor a global holds.

Case ([0262](0262-when-rust-and-js-disagree.md)): N. It's a binding's
shape: the JS is what the program names, `new window.ErrorEvent(..)`,
where a global `new ErrorEvent(..)` constructed another class.

## Context

React's `shared/reportGlobalError` dispatches an error on the window
where the host has no `reportError`:

```js
if (
  typeof window === 'object' &&
  typeof window.ErrorEvent === 'function'
) {
  const event = new window.ErrorEvent('error', {bubbles: true, cancelable: true, message, error});
  const shouldLog = window.dispatchEvent(event);
```

webapi bound a class's constructor once, its module's, `error_event::new`,
the global `new ErrorEvent(..)`; and `js::is_defined(error_event::new)`
asked `typeof ErrorEvent`. Where the window is the global object, as in a
page, `window.ErrorEvent` is `ErrorEvent`. Where it isn't, it's another
class or none: React's server-integration tests run in Node, with
`global.window` a jsdom window, whose `ErrorEvent` Node's global scope
hasn't. The port asked of the global, took the Node branch where upstream
dispatched on the window, and seven react-dom suites failed. Nothing let a
program name a window's own class, as TypeScript's `Window & typeof
globalThis` does, `window.ErrorEvent`.

## Decision

**`new X` of a method is `new this.X(..)`**, as `f` of one is
`this.f(..)` and `get x` `this.x`: the class its receiver holds, each part
of a path a member, `new this.WebAssembly.Module(..)`.

**webapi gives `Window` each constructor its global scope has**, a method
of its module's name: `error_event::new(..)` is `new ErrorEvent(..)`, and
`window.new_error_event(..)` is `new window.ErrorEvent(..)`, of the same
parameters, for every class WebIDL exposes on a window
(`[Exposed=Window]`, `[Exposed=*]`), generated, not listed.

**`js::is_defined` of one asks the class the window holds**, a method of
the one global its type is (ADR 0597): `js::is_defined(Window::new_error_event)`
is `typeof window !== "undefined" && typeof window.ErrorEvent !==
"undefined"`. In a path of bindings (ADR 0624) a constructor is the last
step, what's asked; a step read past it is refused, as what it makes
isn't read of it.

```rust
if js::is_defined(Window::new_error_event) {
    let event = window.new_error_event("error", Some(ErrorEventInit { .. }));
    let should_log = window.dispatch_event(event);
```

```js
if (typeof window !== "undefined" && typeof window.ErrorEvent !== "undefined") {
  const event = new window.ErrorEvent("error", { .. });
  const shouldLog = window.dispatchEvent(event);
```

## Why

- **It's the form there was**, extended: a method's `link_name` is read
  of its receiver, and `new` was the one form that wasn't. No new
  attribute, macro or type.
- **One rule for every class**: the generator writes it from WebIDL's
  `[Exposed]`, 226 constructors, as TypeScript's `typeof globalThis` has
  each, so `window.new_text(..)` and `window.new_url(..)` are guessed from
  one example.
- **Two jobs, not two ways**: the global scope's class, `error_event::new`,
  and a window's, which differs where the window isn't the global object:
  jsdom's, a frame's `contentWindow`. `instanceof` tells them apart.

## Alternatives

- **The constructor as a value**, `window.error_event()` of a type of its
  own, then `new` of it: a second type for every class, and `new` through
  `js::new`, untyped. Rejected: a type per class for one call.
- **A free function of a `this` parameter**, `error_event::new_in(window,
  ..)`: the form for another crate's type, which `Window` isn't; it reads
  as no method TypeScript has.
- **A `Window` method by hand for `ErrorEvent`**: a one-off, which the
  next class asked of a window would repeat. Rejected for the rule.

## Consequences

- `Window` has 226 methods more, each `new_` and its module's name; the
  generator fails where a window's own member already has the name.
- `typeof window === 'object'` and `typeof window.ErrorEvent ===
  'function'` are `!== "undefined"` as every `js::is_defined` is (ADR
  0597): they differ only where `window` is a primitive or a function, or
  its `ErrorEvent` no function, which no host and none of React's tests
  make.
- A window's static members, `window.Notification.permission`, and
  `instanceof` of a window's class aren't bound: the next program that
  needs one extends this rule.

## What holds it

- `test/bindings.test.ts`, "a new binding of a method constructs what its
  receiver holds": a member, a path, and a receiver that's a call, `new
  (globalThis.realmOf(name).Point)(3)`.
- `test/webapi-bindings.test.ts`, "a window's constructor is the window's,
  apart from the global one": a window that isn't the global object, of
  an `ErrorEvent` that isn't Bun's, dispatched on; and a path read past a
  constructor refused. "each class a window has is constructed of the
  window too": every module's `new` has its `Window` method.
- Mutations `new-of-a-method-the-global-class`,
  `defined-of-a-windows-constructor-refused`,
  `defined-path-read-past-a-constructor`.
