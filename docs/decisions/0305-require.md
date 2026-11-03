# 0305. A binding marked `require` is read through `require(module)`

Status: Accepted. Amends [0028](0028-js-module-imports.md): how a module's
export is read.

Case: A ([0262](0262-when-rust-and-js-disagree.md)).

Amended below: a `require` no bundler sees is read off Node's `module`,
`node::module[requireString]`, and called by `js::function::call`. Amended
below (rd684): a `use` of a library's function marked `#[rust_js::require]`
reads it through `require(module)` where the module reads it.

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

## Amendment: the node crate binds `require.resolve` (rd681)

Case ([0262](0262-when-rust-and-js-disagree.md)): N, Node's own call.

React's ReactFlightWebpackPlugin.js finds its Flight client's file where
its module loads, `const clientFileName =
require.resolve('../client.browser.js');`, a CommonJS module's own
`require`, @types/node's `NodeJS.Require`, whose `resolve(request,
options?)` gives the full path of the file `require(request)` would load,
without loading it. The node crate bound `module` and its `require`
method, not `require.resolve`, so a port had no way to write it.

**`node::require::resolve(request, options)` is `require.resolve(request,
options)`**, a binding of the global's member as `process.cwd` is,
`#[rust_js::link_name = "require.resolve"]`: its options,
`RequireResolveOptions { paths }`, a dictionary whose `None` is left out,
`require.resolve(request)` of `None`. What it throws of a module it can't
find, `MODULE_NOT_FOUND`, is a panic, as a binding's throw is; so is a
call in an ES module, which has no `require`, as a read of `node::module`
there is. Its module, `require`, is CommonJS's global's: `require.cache`,
`require.extensions` and `require.main` aren't bound yet, as
`node/coverage.txt`'s `require 1 of 4` counts (`node/coverage.ts` counts
the global's members as `process`'s are).

- Rejected: a `#[rust_js::require]` binding of `resolve`, which reads a
  module's export through `require(module)`, not a member of `require`;
  and `module.require.resolve`, which `NodeJS.Module` doesn't declare.
- Held by `test/node-bindings.test.ts`'s "require.resolve finds a module's
  file from the module it's in", run in Node where `require` is the
  module's own, its `paths` given and left out and of a module it can't
  find; and `test/node-coverage.test.ts`. A binding has no compiler code to
  mutate: its test is what fails where its `link_name` or its options'
  keys are wrong.
## Amendment: a CommonJS hook's `Module` and a module's `exports`

React's Flight Node register is a CommonJS hook: `const Module =
require('module')`, its `Module.prototype._compile` replaced by a function
that passes on all it's given, `originalCompile.apply(this, arguments)`,
and a module's `this.exports` read and set; and it reads the package's own
server entry when it's called, `require('react-server-dom-webpack/server')`
inside `register()`, so it throws outside `--conditions react-server` only
then. Of these, the node crate bound none:

- **`node::module::MODULE`, the `module` module's default export**,
  @types/node's `export = Module`, imported `import Module from "module"`
  (`#[rust_js::name = "Module"]`, as the type's tuple-struct constructor
  holds the Rust name): a `&'static js::Class`, whose `prototype()` is
  `Module.prototype`. `_compile`, which @types/node doesn't declare, is
  read and set by `js::get` and `js::set`.
- **`Module::exports()` and `Module::set_exports(..)`**, `module.exports`,
  @types/node's `exports: any`: any value, `None` of `undefined`.

A require at the point it's read is this ADR's `#[rust_js::require]`, as a
getter of the export, `#[link_name = "get react-server-dom-webpack/server#registerServerReference"]`,
read where `register()` reads it: no bare `require` binding is added beside
it (one way to read a module through `require`), and `node::module`'s
`require` stays the one a bundler doesn't see. Case N: each read, call and
write is the JS upstream's is. test/node-bindings.test.ts, "Module's
prototype is patched and a module's exports read and set", runs it in
Node: a "use client" module's exports replaced, a "use server" one's read
after its code runs, and a plain one compiled by the original `_compile`.

## Amendment: a required module's namespace is the module (#572)

Case ([0262](0262-when-rust-and-js-disagree.md)): R, a result that
changed: the JS read `undefined` where the Rust reads the module.

React's ReactDOMRootFB reads www's module whole, `const
ReactFiberErrorDialogWWW = require('ReactFiberErrorDialog');`, then checks
its `showErrorDialog`. Its binding is the guide's module-namespace path,
`#*` (ADR 0256), read through `require`:
`#[link_name = "get ReactFiberErrorDialog#*"] #[rust_js::require]`. It
printed `require("ReactFiberErrorDialog")["*"]`, the property `"*"` of the
module, `undefined`, so every www root threw a `TypeError` where it loaded.
It's as old as this ADR (c3b910366): a required export was read as
`require(module)`'s property, and `*` is no property but the module.

**`#*` is the module itself through `require` too, as it is through
`import * as`**: `require("ReactFiberErrorDialog")`, and a path past it
the module's properties, `#*.options.title`'s
`require("ReactFiberErrorDialog").options.title`. What `require` reads of
an export is `bindings::required`, beside `js_import`, which reads the
path: one owner of what a path's `#*` means.

- The other readers of `#*` are each right but one: an import's is
  `import * as`, a re-export's `export * as`, and a `#[rust_js::types]`
  of one is refused, as it names no type. `js::import!` of a getter
  binding is wrong of any export, `#*`'s too, which is filed apart:
  it isn't what `require` reads.
- Held by test/bindings-require.test.ts's "a required binding of a whole
  module is the module", run in Node of a CommonJS module: the binding is
  `require(..)`'s module, its method called and a path past it read, and
  the `.d.ts` its `#[rust_js::types]`; and the mutation
  `required-namespace-read-as-property`.
## Amendment: a `use` of a library's function (rd684)

Status: the user's decision (2026-11-03, rd684).

React's FizzTestUtils reads React's `createElement` where it calls it,
because a test's `jest.resetModules()` loads React anew between tests,
and the element must be the React the test renders with:

```js
children.push(
  require('react').createElement(node.tagName.toLowerCase(), props),
);
```

The port's `use react_isomorphic::create_element;` imports React as the
module loads, `import {createElement} from 'react'`, the first React; the
one form that read it at its call was a `#[rust_js::require]` binding of
`react#createElement` typed by hand, a second declaration of React's own
function, which the `react_isomorphic` crate is.

**`#[rust_js::require]` on a `use` of a library's function reads it
through `require(module)` where the module reads it**: the import the
`use` would make, its module and its export, read at each call, never
imported. The same attribute, on the one thing that names the import.

```rust
#[cfg_attr(rust_js, rust_js::require)]
use react_isomorphic::create_element;

children.push(create_element(&tag, props));
```

```js
children.push(require("react").createElement(tag, props));
```

- **Each read in the module**, through the `use`'s name or a path,
  `element::create_element(..)`, is `require(module).name`: one module,
  read one way. Another module's `use` of it imports it as before.
- **At each call, in order**: the `require` and the member are the call's
  callee, which JS reads before the arguments, where the call is, as
  upstream writes it. Rust has no module load to place: the attribute
  asks for JS's, and its function is the same function whichever load
  gave it. A value of it, `let f = create_element;`, reads it where the
  value is made.
- **A function that reads its `this`** (ADR 1081) is called with none,
  `(0, require("m").f)()`, not the module's exports.
- `require` is a name no local of the crate takes, as for a binding.
- **Refused, saying what to write**: on a group or a glob, which names
  more than one; on a `pub use`, the module's `export .. from`, which JS
  loads as the module loads; on a `use` of a binding, whose own
  `#[rust_js::require]` reads every read of its export so; on a `use` of
  the crate's own function, whose module loads with the crate; and on a
  `use` of anything but a library's function.

Case ([0262](0262-when-rust-and-js-disagree.md)): **None**, the user's
decision (2026-11-04): Rust has no module load to order, so where the load
falls against the call's arguments, before them, as JS reads a callee, is
no difference Rust can see; the function called is the same function
whichever load gave it.

Held by `test/require.test.ts`: a library's function read through
`require` at each call, each call in a function, two in one expression,
through a module path and of a function that reads its `this`, a local
named `require` renamed; run in Bun, its module loaded where it's first
called, not where the consumer loads, and loaded anew once its cache is
cleared, as `jest.resetModules()` does; another module's import of it;
and each refusal. Mutations `required-use-unread`,
`required-use-group-kept`, `required-pub-use-kept`,
`required-binding-use-unsaid`, `required-own-use-unsaid`
(`analysis/naming.rs`), `required-use-imported`,
`required-use-namespace-path` (`items.rs`) and
`required-use-name-unreserved` (`analysis.rs`).
