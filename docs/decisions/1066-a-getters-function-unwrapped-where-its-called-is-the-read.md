# 1066. A getter's function unwrapped where it's called is the read

Status: Accepted: the user's decision (2026-11-02, rd519), an exception of
[0262](0262-when-rust-and-js-disagree.md)'s, listed in its Consequences.
Narrows [0012](0012-panics-and-runtime-helpers.md)'s `$unwrap` where a
binding getter's value is only called.

Case ([0262](0262-when-rust-and-js-disagree.md)): rare and harmless, the
user's decision. Where the getter gives `undefined`, JS's call throws a
`TypeError` where Rust's `unwrap()` panics `called \`Option::unwrap()\` on
a \`None\` value`: both throw there, and only the error's message and type
differ.

## Context

React's Flight client creates a task where the host has
`console.createTask`, which Firefox, Safari and jsdom haven't, so its
binding is `Option`al (ADR 0811):

```js
const supportsCreateTask = __DEV__ && !!console.createTask;
..
const createTaskFn = (console: any).createTask.bind(console, getTaskName(element.type));
..
rootTask = console.createTask('"use ' + rootEnv.toLowerCase() + '"');
```

Its port writes what Rust does, the `Option` unwrapped where it's used,
behind `SUPPORTS_CREATE_TASK`:

```rust
let create_task_fn = function::bind(console::create_task().unwrap(), Some(&console), (name,));
this._debug_root_task.set(Some(console::create_task().unwrap()(&root_task_name)));
```

and each `unwrap()` was `$unwrap`'s:

```js
const createTaskFn = $unwrap(console.createTask).bind(console, ..);
const value = $unwrap(console.createTask);
this._debugRootTask = value(..);
```

## Decision

**An `unwrap()` of what a binding's getter gives, read only as the callee
of the call it's made for or as the `this` of a binding's method, is the
getter's read itself** (`body_queries/getter_unwraps.rs`, its THIR's
shape; `std_types/option.rs`'s `Unwrap`):

```js
const createTaskFn = console.createTask.bind(console, ..);
this._debugRootTask = console.createTask(..);
```

- **Which**: a call of `Fn`, `FnMut` or `FnOnce` whose callee is the
  `unwrap()`, through its borrow, `create_task().unwrap()(name)`, or a
  binding's method whose `this` is, `function::bind(create_task().unwrap(),
  ..)`; the `Option` a getter's call itself, `link_name = "get .."`. Not an
  `expect(..)`, whose message the program wrote, nor one kept, returned or
  given anywhere else: `let f = console::create_task().unwrap();` keeps
  `$unwrap`.
- **What differs** (semantics.md): of a `None`, JS's call of `undefined`
  or its `.bind` throws a `TypeError`; Rust's `unwrap()` panics. Both
  throw where Rust's would, so code after it never runs, and a
  `catch_unwind` catches either; only its message and type differ. The
  getter's function called as a member, `console.createTask(..)`, is
  given its object as `this`, as upstream calls it.

## Why

- **It's upstream's JS**: `console.createTask.bind(console, ..)` and
  `console.createTask(..)`, behind the test that it's there, which the
  program writes too.
- **The difference is the error's**: Rust's panic and JS's `TypeError` are
  both a throw at the same place; no value differs.

## Alternatives

- **Keep `$unwrap`**: Rust's message, and upstream's JS lost at four sites
  of the Flight client. Rejected by the user.
- **An `expect`-like attribute on the getter**: a second way to say what
  `unwrap()` says. Rejected.

## What holds it

- `test/js-globals.test.ts`'s "a getter's function unwrapped where it's
  called is the read" (each form, one kept, its JS run in Node, and of a
  `None`, a throw).
- The guide's `semantics.md` entry, and ADR 0262's Consequences.
