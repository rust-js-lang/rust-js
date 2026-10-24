# 0305. A binding marked `require` is read through `require(module)`

Status: Accepted. Amends [0028](0028-js-module-imports.md): how a module's
export is read.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

Amended below: a `require` no bundler sees is read off Node's `module`,
`node::module[requireString]`, and called by `js::function::call`.

## Context

A binding of a module's export, `#[link_name = "pkg#name"]`, is imported:
`import { name } from "pkg"`. react.dev's runESLint, an ES module its
bundler builds, reads a CommonJS package's export through `require`:

```ts
const reactRules = require('eslint-plugin-react-hooks').rules;
```

And a getter of a module's export, `get pkg#name`, was no import at all:
lowering read a local nothing had imported.

## Decision

**A binding marked `#[rust_js::require]` is read through `require(module)`
where it's read, and nothing imports its export; a getter of a module's
export, `get pkg#name`, is imported as a function's is.**

```rust
unsafe extern "Rust" {
    #[link_name = "get eslint-plugin-react-hooks#rules"]
    #[rust_js::require]
    safe fn rules() -> &'static ReactHooksRules;
}
thread_local! {
    static reactRules: &'static ReactHooksRules = rules();
}
// const reactRules = require("eslint-plugin-react-hooks").rules;
```

- An export one binding marks is read through `require` by each binding
  of it: one module, read one way.
- `require` is a name no local of the module takes.

## Why

- **It's the JavaScript a person writes**: the original's `require(..)`.
- **It's the same program**: its bundler, or Node, loads the module
  where `require` asks for it, as the original's does.

## Amended by

- [0373](0373-one-binding-form.md): A binding of a JS function is an ordinary function, and a JS type a struct

## Amendment: a require no bundler sees

React's `shared/enqueueTask.js` wants Node's own `setImmediate`, which
fake timers don't replace, without a bundler seeing a `require`, which
would bundle a Node polyfill; in a browser, it falls back to
`MessageChannel` in a `catch`:

```js
const requireString = ('require' + Math.random()).slice(0, 7);
const nodeRequire = module && module[requireString];
enqueueTaskImpl = nodeRequire.call(module, 'timers').setImmediate;
```

A `#[rust_js::require]` binding is a `require` a bundler sees, which is
what this one mustn't be. So there's no binding of it, and no attribute:
**the one way is upstream's, of what's bound already and two bindings
more**, each a plain JS value or member:

- **`node::module`, Node's `module`**, `NodeJS.Module`, a static of the
  node crate, of its `id`, `filename`, `loaded` and `require`: in an ES
  module, which has none, reading it throws a `ReferenceError`, a panic.
- **`js::function::call(f, this_arg, args)`, `f.call(thisArg, ...args)`**,
  `Function.prototype.call`, a member of `js::function` (ADR 0740).
- The rest is what's there: `js::get` of a name the program makes,
  `module[requireString]`, an `unsafe { js::cast(..) }` of what TypeScript
  types `any` there (Flow's `$FlowFixMe`), and `catch_unwind`, whose
  `if let Err(_err)` is the `try`'s `catch (_err)` (ADR 0706).

```rust
if ENQUEUE_TASK_IMPL.get().is_none()
    && let Err(_err) = catch_unwind(|| {
        let require_string = js::string::slice(&format!("require{}", js::shown(js::math::random())), 0.0, Some(7.0));
        let node_require: &dyn Fn(&str) -> Option<&'static Unknown> =
            unsafe { js::cast(js::get(js::unknown(node::module), &require_string)) };
        let timers: &Unknown = unsafe { js::cast(function::call(node_require, Some(&node::module), ("timers",))) };
        ENQUEUE_TASK_IMPL.set(Some(unsafe { js::cast(js::get(timers, "setImmediate")) }));
    })
{
    // we're in a browser
```
```js
if (!enqueueTaskImpl) {
  try {
    const requireString = `require${Math.random()}`.slice(0, 7);
    const nodeRequire = module[requireString];
    const timers = nodeRequire.call(module, "timers");
    enqueueTaskImpl = timers.setImmediate;
  } catch (_err) {
    // we're in a browser
```

Upstream's `module &&` isn't written: TypeScript types `module` as a
`Module`, never falsy, and where there's none, its read throws, which the
`catch` takes as upstream's does. `('require' + Math.random())` is a
template, `` `require${Math.random()}` ``, the same text.

Case N: each read and call is the JS upstream's is. `test/node-bindings.test.ts`,
"React's enqueueTask reads Node's setImmediate past a bundler, else posts
a message", runs both paths in Node: a CommonJS module's `module`, whose
`setImmediate` is the real one past a faked global, and an ES module's,
which posts a message.
