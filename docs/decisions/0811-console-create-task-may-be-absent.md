# 0811. `console.createTask` is a property the host may lack

Status: Accepted. Extends [0597](0597-whether-the-host-has-a-global-is-is-defined.md):
a member no TypeScript lib declares.

Case ([0262](0262-when-rust-and-js-disagree.md)): none: the program reads
what the host has, `None` where it hasn't it, as JS does.

## Context

React's DEV builds tag each element with a DevTools task, where the host
has V8's `console.createTask`:

```js
const createTask =
  __DEV__ && console.createTask
    ? console.createTask
    : () => null;
..
debugTask: console.createTask ? console.createTask('lazy()') : null,
```

Chrome and Node have it; Firefox, Safari, jsdom and Bun don't. No WebIDL
specifies it and no TypeScript lib declares it, so the webapi crate, made
of WebIDL as TypeScript's DOM is (ADR 0024), had no `console::create_task`.
React's Flow declares it on `console`, `createTask(label: string):
ConsoleTask`, with `interface ConsoleTask { run<T>(f: () => T): T }`.

ADR 0597 made `js::is_defined(binding)` the one way to ask whether a host
has a global TypeScript declares, as always there: reading a missing
global throws. A property of `console` doesn't: missing, it reads
`undefined`, as an optional property's does.

The user's decision: bind it, as a property that may be absent.

## Decision

**A member no TypeScript lib declares, which some hosts have, is declared
as an optional property, an `Option` of its value**, as TypeScript
declares one, `createTask?: (label: string) => ConsoleTask`: `None` where
the host hasn't it. The webapi crate's generator writes it by hand, in its
`EXTRA` members, now of a namespace too, and the type its function makes
in `TYPES_BY_HAND`, each typed as React's Flow declares it, and each
saying no TypeScript lib does:

```rust
// webapi
pub struct ConsoleTask(PhantomData<JsObject>);

impl ConsoleTask {
    #[cfg_attr(rust_js, rust_js::link_name = "run")]
    pub fn run<T>(&self, f: impl FnOnce() -> T) -> T { .. }
}

pub mod console {
    #[cfg_attr(rust_js, rust_js::link_name = "get console.createTask")]
    pub fn create_task() -> Option<&'static dyn Fn(&str) -> &'static ConsoleTask> { .. }
}
```

A program asks as React does, by the value:

```rust
pub fn debug_task() -> Option<&'static ConsoleTask> {
    console::create_task().map(|create_task| create_task("lazy()"))
}
```

```js
export function debugTask() {
  const createTask = console.createTask;
  return createTask ? createTask("lazy()") : undefined;
}
```

`ConsoleTask`'s `.d.ts` is Flow's interface, written out, `{ run<T>(f: ()
=> T): T }`, as there's no TypeScript name for it.

## Why

- **What JS reads**: `console.createTask`, `undefined` where the host
  hasn't it, which never throws, so it's the value's `Option`, as every
  optional property is, `Error.prepareStackTrace` among them.
- **ADR 0597's rule, a declaration as TypeScript's would be**: a global
  TypeScript declares as there is called as there, and asked of by
  `js::is_defined`; a member no lib declares, rust-js's crate declares as
  TypeScript declares what a host may lack, an optional property, not a
  method every host has. Each binding has one declaration, so a program
  asks of each one way.
- **React's JS**: `console.createTask ? console.createTask(..) : null`,
  not `typeof console !== "undefined" && typeof console.createTask !==
  "undefined"`.

## Alternatives

- **A method, `console::create_task(label)`, asked of by
  `js::is_defined(console::create_task)`**, as ADR 0597 asks of a
  TypeScript-declared member: it would declare every host has it, which
  no declaration says, and React's `console.createTask ? .. : ..` would be
  two `typeof`s.
- **`createTask`'s task an `Option` of the function's result too**, so
  React's `() => null` fallback has its type: a type looser than Flow's,
  which says it always makes one.

## Consequences

- React's `const createTask = __DEV__ && console.createTask ?
  console.createTask : () => null;` has no one Rust type for its two
  values, a `dyn Fn(&str) -> &ConsoleTask` and a closure that gives
  `None`. Its port holds the `Option` and calls it where it's there,
  `let createTask; if (__DEV__) { createTask = console.createTask; }`
  and `createTask ? createTask(name) : undefined`; a closure that forwards
  to it, `move |name| Some(create_task(name))`, is a function of its own,
  `(name) => createTask(name)`, not `console.createTask` itself.

## What holds it

- `test/webapi-bindings.test.ts`, "console.createTask is a function the
  host may lack, and its task runs what it's given": read once in DEV, a
  module's `let`, absent in Bun and present as a host gives it, its task
  running what it's given.
- `test/webapi-coverage.test.ts`: `webapi/src/lib.rs` is what
  `webapi/generate.ts` writes.
- The guide's [global scope](../guide/interop/global-scope.md#a-global-some-hosts-lack).
