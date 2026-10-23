# 0648. V8's stack trace API is the node crate's, as @types/node types it

Status: Accepted. Amended by
[0653](0653-a-js-function-value-is-a-dyn-fn-of-its-signature.md): `Error.captureStackTrace`
is bound, its `Function` a `&dyn Fn()`. Extends [0272](0272-node.md) and
[0351](0351-node-coverage.md), whose measure counts `Error`'s members
too. Amends [0443](0443-a-value-a-caller-may-leave-out-is-an-option.md):
a setter's `None` is no optional argument.

Case ([0262](0262-when-rust-and-js-disagree.md)): N: each is the JS
it binds, read and written as JS reads and writes it.

## Context

React reads its owner stacks through V8's stack trace API, which Chrome
and Node have. `shared/ReactOwnerStackFrames.js` sets the hook that writes
an error's `stack`, reads the stack, and puts the hook back:

```js
const prevPrepareStackTrace = Error.prepareStackTrace;
Error.prepareStackTrace = DefaultPrepareStackTrace;
let stack = error.stack;
Error.prepareStackTrace = prevPrepareStackTrace;
```

`DefaultPrepareStackTrace` is `undefined` in a browser's build, `export
default undefined as any as (Error, Array<CallSite>) => string`, and in a
server's `DefaultPrepareStackTraceV8`, a hook of its own that writes each
frame's `callSite.toString()`. `ReactComponentStackFrame.js` saves and
sets the hook too, React Flight's client saves and sets
`Error.stackTraceLimit`, and DevTools reads a frame's `getFileName()` and
`getLineNumber()`. None of it was bound, and a setter's `None` was taken
for an optional argument left out, so `Error.prepareStackTrace =
undefined` was refused, "the `#[link_name]` with this signature".

## Decision

**V8's stack trace API is the node crate's, typed as `@types/node`'s
`globals.d.ts` types it: `node::js_error`, `Error`'s statics it adds, and
`node::CallSite`, `NodeJS.CallSite`.**

| Rust | JS |
|---|---|
| `js_error::prepare_stack_trace()` | `Error.prepareStackTrace` |
| `js_error::set_prepare_stack_trace(hook)` | `Error.prepareStackTrace = hook` |
| `js_error::stack_trace_limit()`, `set_stack_trace_limit(n)` | `Error.stackTraceLimit`, `= n` |
| `call_site.to_string()`, `get_file_name()`, `get_line_number()` .. | `callSite.toString()`, `getFileName()`, `getLineNumber()` .. |

- **The hook is a JS function, `&'static dyn Fn(&JsError, &[&CallSite])
  -> R`**, as a function JS gives is where its signature is known,
  `Reflect`'s descriptor's `get`, a state's setter (ADR 0367); `None` is
  `undefined`. What it gives is `any`: of one JS gives, `Option<&'static
  Unknown>`, as an `any` result is; of one Rust gives, any `R`, as an
  `any` argument is any value. So what the getter gave is set back as it
  is, and so is a Rust function, `Some(&prepare_stack_trace)`, its JS
  `prepareStackTrace`. A bare `None` names its `R`,
  `set_prepare_stack_trace::<String>(None)`, as a `None` of a generic
  type does.
- **It's an `Option`**, though `@types/node` 24 declares it a method: V8
  has no hook until one is set, React's `DefaultPrepareStackTrace` is
  `undefined`, and a binding declares what JS gives.
- **A frame is `&[&CallSite]`**, a JS array of JS objects, as webapi's
  `MutationObserver`'s callback is given `&[&MutationRecord]`. Its numbers,
  a line, a column, an offset, are `u32`, and a `null` one `None`.
  `getFunction()` is `&'static JsObject`, as webapi's generator types a
  function JS gives whose signature isn't said. `toString()` is
  `Object`'s to TypeScript, and V8's own: bound, as React calls it.
- **`DefaultPrepareStackTrace` is a `const` of the hook's type**, the
  module's default: `js::export_default! { pub const
  DEFAULT_PREPARE_STACK_TRACE: Option<&'static dyn Fn(&JsError,
  &[&CallSite]) -> String> = None; }` is `export default undefined;` (ADR
  0625).
- **A setter's value is never an optional argument**:
  `is_optional_param` says no of a `set` binding's, so its `None` is
  written, `Error.prepareStackTrace = undefined`. Before, it was left out,
  and the setter refused for a missing value.
- **The node crate's measure counts `Error`'s members**, those
  `globals.d.ts` adds to `ErrorConstructor`, as it counts `process`'s and
  `console`'s, from one list of the globals it counts.

## Why

- **One rule says which crate binds what**: each binds what its
  TypeScript declares, the js crate ES2024's `lib.es*.d.ts`, webapi the
  DOM's, node `@types/node`'s. ECMAScript has no stack trace API, and
  TypeScript's libs don't declare it; `@types/node` does. A browser
  program that uses it names the node crate, a declaration only: what it
  writes is `Error.prepareStackTrace`, no import, which Chrome has.
- **The JS is React's**, line for line: the hook read, set, the stack
  read, the hook put back.
- **One type for a JS function value**, the one rust-js has: a function
  JS gives of a known signature is a `&'static dyn Fn`, which a Rust
  function is given as.

## Alternatives

- **The js crate, beside `JsError::stack`**: where a reader looks first,
  but a second rule for where a binding goes, a crate's TypeScript and
  "or what V8 has", and the js crate's measure, ES2024's, wouldn't count
  it.
- **A crate of V8's own**: a crate of four bindings, for what
  `@types/node` already declares.
- **`Option<fn(..) -> R>`**: a Rust `fn` pointer is a function JS has too
  (ADR 0602), but no binding gives one, and a JS hook may be a closure.
- **The hook's result `String`**, what React's give: narrower than `any`,
  wrong of the hook another script set, read and called, which V8 lets
  give any value as the stack.
- **The hook's result `Option<&'static Unknown>` both ways**: a Rust hook
  would give an `Unknown` of its text, not the text.

## Consequences

- **`Error.captureStackTrace(target, constructorOpt?)` isn't bound**: its
  `constructorOpt` is TypeScript's `Function`, any function, which no Rust
  type takes yet, `None` included. React calls it in its test utilities
  and DevTools only. The measure lists it `-`.
- `NodeJS.CallSite` isn't counted by the measure, which counts no type of
  `namespace NodeJS`.
- A `const` default, imported, is named as the `const` is,
  `DEFAULT_PREPARE_STACK_TRACE`, where React's import names it
  `DefaultPrepareStackTrace`.

## What holds it

- `test/node-bindings.test.ts`, "V8's stack trace hook is saved, set and
  restored": `formatOwnerStack`'s save, set and restore, the V8 hook as
  its module's default, set, a hook of `getFileName()` and
  `getLineNumber()`, `undefined` set, and `stackTraceLimit`, each as JS
  and run in Node.
- `node/coverage.txt`: `Error#prepareStackTrace` and
  `Error#stackTraceLimit` bound.
- Mutation `setter-value-optional`.
