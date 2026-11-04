# 1083. React Native's host globals are the react-native crate's

Status: Accepted (2026-11-04, the user's decision, #562). Extends
[0102](0102-js-and-webapi.md), whose crates are each a host's or a
library's bindings, and [0272](0272-node.md), Node's.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: each binding is the
JS of the global it reads, as upstream reads it.

## Context

React's `scheduler/src/forks/SchedulerNative.js`, React Native's fork of
the scheduler, reads a global React Native's C++ runtime installs
(`RuntimeSchedulerBinding.cpp`), and Scheduler's own where it's absent:

```js
declare const nativeRuntimeScheduler: void | NativeSchedulerType;

export const unstable_NormalPriority: PriorityLevels['NormalPriority'] =
  typeof nativeRuntimeScheduler !== 'undefined'
    ? nativeRuntimeScheduler.unstable_NormalPriority
    : Scheduler.unstable_NormalPriority;
```

rust-js bound no global of React Native's, in `js`, `webapi` or `node`,
and a port's local `#[link_name]` stand-in is a hidden gap (rd709), so the
file stayed upstream's JS.

## Decision

**React Native's host globals are a binding crate of their own,
`react-native/`, beside `webapi` (the browser's) and `node` (Node's)**: lib
`react_native`, Cargo's `rust-js-react-native`, npm's
`@rust-js/react-native`, using `js` alone, one row of the catalog,
`tooling/crates.js`, which its packaging, the hosts and the tests read.
It starts with `nativeRuntimeScheduler`:

- **The global, a static that may be absent**: `#[link_name =
  "nativeRuntimeScheduler"] pub safe static native_runtime_scheduler:
  &'static NativeScheduler`, as webapi's `performance` is; `js::is_defined(
  native_runtime_scheduler)` is `typeof nativeRuntimeScheduler !==
  "undefined"`, upstream's test.
- **Its type, Flow's `NativeSchedulerType`**, of eleven members: the five
  priorities, getters, `get unstable_NormalPriority`, each a `u8`, the
  scheduler's `PriorityLevel`, 1 to 5; `unstable_scheduleCallback`,
  `unstable_cancelCallback`, `unstable_getCurrentPriorityLevel`,
  `unstable_shouldYield`, `unstable_requestPaint` and `unstable_now`,
  methods, each read as a value, as upstream exports them, by
  `js::method((native_runtime_scheduler, NativeScheduler::unstable_now))`,
  `nativeRuntimeScheduler.unstable_now`.
- **Honest types of the host's own (the user's choice T1)**: Flow types
  `unstable_scheduleCallback` with Scheduler's `Callback` and `Task`, an
  unchecked `declare`, where the host's task is its own object. The crate
  can't name a program's types, and doesn't pretend to: its `Task` is an
  opaque JS object, which only `unstable_cancel_callback` takes, and its
  `Callback` is `#[repr(transparent)] Callback(Box<dyn FnMut(bool) ->
  Option<&'static js::Function>>)`, Flow's `(didTimeout: boolean) =>
  ?Callback`, its continuation a function of any signature. A port that
  holds the host's function and Scheduler's under one type, as upstream's
  Flow does, writes that one assertion itself, `unsafe { js::cast(..) }`,
  which prints nothing.

```rust
thread_local! {
    pub static UNSTABLE_NOW: Option<&'static dyn Fn() -> f64> = if js::is_defined(native_runtime_scheduler) {
        Some(js::method((native_runtime_scheduler, NativeScheduler::unstable_now)))
    } else {
        None
    };
}
```
```js
export const unstableNow =
  typeof nativeRuntimeScheduler !== "undefined" ? nativeRuntimeScheduler.unstable_now : undefined;
```

## Why

- **One crate per host, as the hosts are**: a browser's globals are
  webapi's and Node's node's; React Native's runtime is another host, with
  globals no other has.
- **Bound as its host defines them**: no member typed of what the host
  doesn't give, so no read of a field of Scheduler's task off the host's
  is possible without the `unsafe` that says so.

## Alternatives

- **In `webapi` or `node`**: neither host has it; a browser program would
  see a global that's never there.
- **rust-js-lang/bindings' `bindings/react-native`**, a crate outside
  rust-js: the host's globals would be versioned apart from the compiler
  that reads them, where `webapi`'s and `node`'s aren't.
- **Generic members typed by the caller, `unstable_schedule_callback<C,
  T>(..) -> T` (T2)**: no cast at the port, but a safe binding that gives a
  program the host's task as its own `Rc<Task>`, whose fields it then
  reads: a wrong answer with no `unsafe`.
- **Every member `&Unknown` or `js::Function` (T3)**: looser than Flow's
  type, a gap (AGENTS.md).

## Consequences

- `@rust-js/react-native` is packaged and published with the others, at
  0.0.1; an app names it `react_native = { package = "rust-js-react-native",
  version = "~0.0.1" }`.
- React's port writes `SchedulerNative.rs`, each export a module-load
  value of the conditional, as upstream's are.
- What else React Native's host installs, `nativeFabricUIManager` say, is
  this crate's when a port needs it.

## What holds it

- `test/react-native-bindings.test.ts`, "React Native's
  nativeRuntimeScheduler is a global that may be absent, read as upstream
  reads it": each member read and called, a member read as a value, run
  against a host's stand-in and with none.
- `test/host-boundaries.test.ts`, "the catalog of crates is each one's
  Cargo.toml, each a binding crate", and `test/crates-io.test.ts`, its
  package.
- The guide's [crates rust-js ships](../guide/libraries/README.md#react_native-react-natives-host).
